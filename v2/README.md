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
- The **old tree's** browser bundle is not built here: its `cargo xtask wasm`
  is a compile check, and it says so (`KNOWN-GAPS.md` §13). (v2's own
  browser door has no bundler to wait for: §78.)
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
  application. So are the **prompt seam** (§45) and the **resumable
  command** (§50): a command that asks the host while it runs.
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
- `crates/craftkit-midi` — the **MIDI library**: the notes, the
  controllers and the Standard MIDI File (§217). A library, not a
  plugin, for the same reason the geometry is: a file is a file, and
  the reader knows nothing about a document.
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
- `crates/craftkit-plugin-prose` — the **prose battery**: the document
  model of a word processor — blocks, styles, media, and the runs that
  are intervals over a text — ported into v2's seams, and the five
  things it found (§49). It is optional, like the others.
- `crates/craftkit-plugin-layout` — the **layout battery**: the document
  model of a page layout application — spreads, pages, items, layers,
  stories and assets — ported into v2's seams, and the five things it
  found (§51). It is optional, like the others.
- `crates/craftkit-control` — the **other door**: a session behind a
  token-authenticated socket, with the family's wire format, so that a
  running program can be driven from another one (§55).
- `crates/craftkit-gates` — the tree's **own gate**: the deny
  attribute, the layering, and what a plugin may depend on. It is a
  **function** over a root, so the tests can point it at trees that are
  broken on purpose and say that it bites (§54).
- `crates/craftkit-numeric` — the **typed value**: a `Length` in
  millimetres and an `Angle` in radians, and the arithmetic over them. A
  **library**, not a plugin: it contributes nothing to the registry and
  the linker does not wire it in (§52).
- `crates/craftkit-plugin-numeric` — the **numeric battery**: the typed
  value as a plugin — lengths, angles and boxes, with the units derived
  from the stored millimetres and radians (§52). It is optional, like
  the others.
- `crates/craftkit-payload` — the **payload reader**: a bounded reading
  of the values a document carries. A **library**, not a plugin: the
  core bounds a command's parameters and says nothing about a payload,
  so the reading of one is a plugin's business and the mechanism is
  shared (§61).
- `crates/craftkit-text-layout` — the **line breaker**: the box, glue and
  penalty model, and the two breakers — the optimal one and the greedy
  one. A **library**, not a plugin: it contributes nothing to the
  registry and the linker does not wire it in (§58).
- `crates/craftkit-font` — the **kit's own typeface**: a table of
  glyphs, each a set of strokes, drawn in code — the same rule the
  icons follow, and for the same reason. A **library**, not a plugin: it
  contributes nothing to the registry and the linker does not wire it in
  (§128).
- `crates/craftkit-wav` — the **WAV reader**: the samples of a sound,
  from the file — the header and the numbers, read within limits. A
  **library**, not a plugin: it contributes nothing to the registry and
  the linker does not wire it in (§133).
- `crates/craftkit-audio` — the **mixer**: the samples of many tracks,
  summed into one. A **library**, not a plugin: it contributes nothing
  to the registry and the linker does not wire it in (§134).
- `crates/craftkit-plugins` — the kit's own **audio plugins**: a
  catalogue of small programs that change a signal — a gain, a low pass
  and a delay. A **library**, not a plugin: it contributes nothing to
  the registry and the linker does not wire it in (§141).
- `crates/craftkit-plugin-text` — the **text battery**: a paragraph, its
  words, the measure it sits in, the **metric** it is set in, the lines
  it breaks into and the **letters** it is drawn with — ported into v2's
  seams, and what it found (§58, §59, §60, §129). It is optional, like
  the others.
- `crates/craftkit-jpeg` — the **JPEG decoder**: the family's own,
  clean-room from ITU-T T.81, ported here whole — the twelve-bit and the
  arithmetic-coded files that the common decoders refuse. A **library**,
  not a plugin: it contributes nothing to the registry and the linker
  does not wire it in (§70).
- `crates/craftkit-image` — the **picture's size**: read from a
  picture's header, in JPEG and PNG. A **library**, not a plugin: it
  contributes nothing to the registry and the linker does not wire it in
  (§68).
- `crates/craftkit-plugin-image` — the **image battery**: a picture is a
  size and a file — a node whose payload holds the size and whose stream
  holds the bytes (§68). It is optional, like the others.
- `crates/craftkit-raster` — the **rasteriser**: the canvas that draws a
  document into a picture, and the PNG writer that carries it. A
  **library**, not a plugin: it contributes nothing to the registry and
  the linker does not wire it in (§67).
- `crates/craftkit-mcp` — the **MCP door**: the registry, spoken to an
  agent, with **one tool per command** (§41). The SDK's **prompt seam**
  (§45) is its other half: the host's turn before a command runs.
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
- `apps/craftkit-repl` — **the terminal door**: a person drives the kit
  from a prompt (§75).
- `apps/craftkit-wasm` — **the browser door**: the kit in a page, as a
  WebAssembly module and the page that loads it. No bundler is involved:
  the module is the file cargo built (§78).
- `apps/craftkit-window` — **the window**: the same registry, the same
  document and the same plan, in a window a person can look at. It is
  thin, and the gate holds it thin (§91).
- `crates/craftkit-clap-abi` — **the CLAP contract**: the shapes a CLAP
  plugin is made of — the version, the entry, the factory and the
  descriptor — and the two names a host looks for. No dependencies and
  no `unsafe` (§76).
- `crates/craftkit-clap` — **the CLAP host**: opens a shared library,
  finds the entry, checks the version, initialises the plugin and reads
  the descriptors. The fourth crate that may hold `unsafe` (§76).
- `crates/craftkit-clap-fixture` — **a reference CLAP plugin**: a real
  `cdylib` that exports the entry by name, so that the host's tests
  open a plugin and not a mock. The fifth crate that may hold `unsafe`,
  and the only thing in it that is unsafe is the attribute that gives
  the symbol its name (§76).
- `crates/craftkit-plugin-clap` — the **CLAP battery**: a plugin in the
  document — a node whose payload holds the plugin's identity and its
  state, the state as base64 and the state as the plugin's own bytes
  (§87). It is optional, like the others.
- `crates/craftkit-vst3` — **the VST3 host**: opens a shared library,
  **calls** the entry the format names, asks the factory what it can
  make, makes an instance and drives it. The sixth crate that may hold
  `unsafe` (§119).
- `crates/craftkit-vst3-fixture` — **a reference VST3 plugin**: a real
  `cdylib` that exports the entry by name, so that the host's tests
  open a plugin and not a mock. The seventh crate that may hold
  `unsafe` (§118).
- `crates/craftkit-plugin-vst3` — the **VST3 battery**: the second
  plugin in the document — an identity, a parameter list and a state,
  with the parameter named and the value in the units the plugin says
  (§120). It is optional, like the others.
- `crates/craftkit-au-abi` — **the AU contract**: the shapes an Audio
  Unit is made of — the four-character codes, the properties and their
  scopes, the shape of the audio and the unit's own function
  signatures. No dependencies and no `unsafe` (§121).
- `crates/craftkit-au` — **the AU host**: opens a bundle, finds the six
  symbols the format names, makes an instance and drives it. The
  **ninth** crate that may hold `unsafe` (§123).
- `crates/craftkit-au-fixture` — **a reference AU**: a real `cdylib`
  that exports the symbols by name, so that the host's tests open a
  unit and not a mock. The **tenth** crate that may hold `unsafe`
  (§122).

**1954 tests.** The MCP server links **all fifteen batteries** in, so the
workspace's count is the same with or without the studio's features
(`craftkit-studio/vector`, `/sound`, `/pdf`, `/film`, `/fx`, `/art`,
`/grid`, `/cad`, `/prose`, `/layout`, `/numeric`, `/text`, `/image`,
`/clap`, `/vst3`) — and the studio
on its own, with no features, carries **238** of them, and **254** with
the text battery. (Both measured; the number in an earlier draft of this
line was stale, and a number that is not measured is a number that is
wrong.)

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
- **The old tree's front ends.** The **MCP server is built** (§41: one
  tool per command, generated from the registry), the **bridge** is built
  (§35), the **terminal door** is built (§75) and the **browser door** is
  built (§78) — and what v2 has besides those is the **projection** every
  front end would read. The **window** is built too (§91: a thin front
  end over the same registry), and the **control channel** is a crate
  with its own tests that the **window opens over its own session**
  (§110) — so an agent that runs a command over the channel changes
  the document the window is drawing.
- **A real document model.** The vector battery (§12) put paths, curves and
  colours in a document that is still a list of nodes with rectangles. What
  the port showed is missing: a typed body, text, and images. The
  **geometry** has a home (§13), the document is a **tree** (§14), and the
  **selection** is the session's state (§14). What is still missing is the
  rest of a document model: images — and even that is nearly there: the
  image battery puts a picture in the document and the snapshot draws it
  (§ the picture is a size and a file). Two of the three are not missing
  any more: the **appearance** is the kit's own (§95) and the **layer**
  is the kit's own too (§103) — a node with no appearance of its own
  takes the layer's, and a hidden layer hides what is on it. **Text** is no
  longer on that list: a paragraph and its runs are in the prose battery
  (§49), and a paragraph **laid out** — broken into lines — is in the
  text battery (§58), and the **metric** it is set in is a node of its
  own (§59).

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
cargo test          # 1954 tests
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

(§125 is the arc that gives the sound document a place again — a
**derived** one, so that it never goes stale.)

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
- **There is no audio thread.** The kit renders a document offline
  (§135) and it has no device and no playhead; a command in v2 runs on the caller's
  thread, which is the wrong thread for a block of audio: a real-time
  door needs its own seam, its own rules about locking and allocation, and
  its own tests. The sound tree has that seam already, and it is the
  reason its plugin trait splits the work into `process` (audio thread),
  `PluginEditor` (control thread) and `PluginStateReader` (control
  thread). v2 has none of it, and this port does not pretend otherwise.
- **The AU plugin format is half here.** The sound tree hosts CLAP, VST3
  and AU binaries: a user installs a plugin and it is in the session, with
  no source change. That is genuinely open there. **CLAP and VST3 are
  here now** (§76–§87 and §116–§120): a host crate, a format to host
  them in, and the decision of what a plugin's state looks like in a
  document whose payload is JSON. **AU**'s contract and its host are here
  too (§121–§123), and what is missing is the **battery**: AU's
  parameters and its state are CoreFoundation's, so there is nothing
  portable for a document to carry yet.
- **The mixer is modelled and drawn.** Ten insert slots and ten send
  slots per strip, a routing graph over buses, and per-track automation
  are a **model**, and the model is where the sound tree's closed enums
  live (`Route`, `AutoParam`, `ChannelFormat`). In v2 they are all
  payload, and the model **is** written: a bus is a node, a send is a
  node with a relation, an insert is a node with a plugin's name, and an
  automation lane is a node with a parameter's name (§20.4). The
  **drawing** is here too (§127): the buses are rows under the tracks,
  the sends are the lines between them, the inserts are the boxes at the
  left of a row and the lanes are the curves on it. What is still
  missing is the **audio** — the mixer is a picture of a mixer, and the
  sound is not made. The **samples** are read now (§133); what is
  missing is the engine that mixes them.
- **The menus are not ported.** The sound tree's menu bar is generated
  from a **catalog file** of 512 leaf paths, and a command whose path is
  not in the catalog never appears in the bar. v2's projection is a view
  of the registry, and there is no catalog; a menu tree is a question for
  the tree.

### What this does not claim

- **No sound was made.** Nothing here decodes audio, mixes it or plays
  it. A source is a path and a length. — **Answered in §133–§135**: the
  sources are read, the clips are mixed and the document is rendered
  into a file. What is still missing is a **device** and a
  **playhead**.
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
- **The AU plugin format.** CLAP, VST3 and AU binaries are genuinely
  open in the sound tree: a user installs one and it is in the session,
  with no source change. **CLAP and VST3 are here** (§76–§87,
  §116–§120) — a host crate each, a format to host them in, and a
  decision about what a plugin's state is in a document whose payload is
  JSON. **AU's contract and its host are here** (§121–§123); what is
  missing is its **battery**, because its parameters and its state are
  CoreFoundation's.
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
- **The AU plugin format.** CLAP and VST3 are ported (§76–§87,
  §116–§120), and **AU's contract and its host** are (§121–§123); what
  is missing is AU's **battery**, and the reason is the format's own:
  its parameters and its state are CoreFoundation's, so a document has
  nothing portable to carry yet.

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
(which belongs to the lane), and the **AU** plugin format (it is FFI, and
FFI is unsafe). The allocation measurement of the lane is no longer in that
list: it is built (§32), and it cost one piece of `unsafe` in a crate of
its own. The **door** for the plugin formats is built too (§34), and all
three formats have come through it: **CLAP** (§76–§87), **VST3**
(§116–§120) and **AU** (§121–§123). What AU has not brought is its
**battery**, and the reason is the format's own.

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
the plugin formats, which are FFI, and FFI is unsafe. All three have come
since — **CLAP** (§76–§87), **VST3** (§116–§120) and **AU** (§121–§123) —
and each landed in its own crate under the same gate, and not in the
middle of the SDK.

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
| `craftkit-clap` | the CLAP host: opening a plugin and reading the entry it exports |
| `craftkit-clap-fixture` | a reference CLAP plugin: the entry point is a data symbol, found by name |
| `craftkit-vst3` | the VST3 host: opening a plugin and calling the factory it exports |
| `craftkit-vst3-fixture` | a reference VST3 plugin: the entry point is a function, called by name |
| `craftkit-au` | the AU host: opening a unit and calling the symbols it exports |
| `craftkit-au-fixture` | a reference AU: the symbols are found by name |

And the rules around it are mechanical: `unsafe` appears **only** in
those nine crates; each of them says `deny(unsafe_code)` and **not**
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

The **formats** are not built. (The door is a **command** now: §35.) There is no VST3 and no AU here:
each is a large ABI in another language, and each would be a **shim**
over this door, not a change to it. What is built is the door, the
contract, the version check, and the proof that a plugin in another
shared library can be reached from a program that has never seen it.
(CLAP came later, in §76: a **shim over the same idea**, with its own
door, because its entry point is a data symbol and not a function. And
in §79 the host **makes** plugins, not only reads them. **VST3 came in
§116–§120**, and it needed the same shim and its own door, because its
entry point is a function and the CLAP one was not.)

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
breaks, UAX #9 bidi) — of which the **line breaker** has been ported
into this tree (§58) —, crash-safe file writing, path safety for paths
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
  more expensive than it needs to be. — **Taken in §53**: the nodes are
  shared now, and the cost of an edit no longer depends on the size of
  the document.
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
  **all nine batteries linked in**, one session, and the protocol on its
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
checking that they are the same size. With all twelve batteries it is
**153**, measured the same way.

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

**§45 built the first half** — the host's turn *before* the command
runs — and **§50 built the second**: a command that asks *while* it
runs, with the session holding the question and the answer.

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
| the **word processor** | a run is an **interval**; the words a run covers are **derived**; a cell that holds a tree is not a payload | nothing |

Nine ports. Six of them cost nothing. The two that did — the artist's
unions and the branches — bought the kit's two strangest features, and
everything the family has taught it since has cost **nothing**.

---

## 45. The prompt seam: the host's turn before the command runs

§44 left one thing open and said so: **the prompt**. CADCraft's
commands may ask — its registry has `interactive: Option<Factory>`, and
its MCP server exposes the prompt itself as a tool, so an agent types at
it exactly as a person would. This kit's rule is the opposite: a command
takes its parameters and never asks.

The rule is a **choice**, and this section is the choice made, with the
shape of the change written down.

### What was built

- a **`Question`** — what is wanted, of what kind, and one line about it,
  taken from the command's own declaration;
- a **`Supplier`** — where the answers come from: a person, an agent, a
  script, a file. `MapSupplier` is the one that already has them;
- **`Session::execute_with(id, params, supplier)`** — run a command, and
  ask the host for the parameters that are missing **before** it runs.

And over the protocol, the MCP server tells an agent what is missing, of
what kind, and that it can call again:

```text
the command `node.add` needs `name` (text): What to call it.
Call it again with it.
```

### The rules it keeps

- **The command does not change.** It declares what it needs; it does not
  ask for it. The host is given a turn; the command is given none.
- **The registry does not change.** No command gains a "prompts" field,
  and no kind becomes interactive. The seam is at the **session**.
- **The document does not change.** A command that is asked for and then
  run is one command, and its undo is one step.
- **A host is asked only for what is missing.** The questions are the
  command's own required parameters that the caller did not give, in the
  order they are declared; the optional ones are never asked for.
- **A host that cannot answer changes nothing.** The parameter stays
  missing and the command fails the way it would have failed anyway — so
  the seam can never make a call succeed that should not have.

### What it is not

It is **not** the CAD case. CADCraft's interactive commands ask *while
they run*: they draw a preview, wait for a point, and keep going until
the person says Enter. That is a **resumable command** — a state
machine per command, with the session holding it — and it is a bigger
thing than this seam. **§50 builds it.**

What this seam covers is the other half, and it is the half that an
agent needs: a command whose parameters are not known before it runs, and
a host that can get them. For an agent over MCP that is a second call
with the value; for a person it is a prompt; for a script it is a file.
The three are the same seam with three suppliers.

### What it cost

A trait, a struct and a method. The `ParamError` gained one word — a
missing value and a wrong value are different questions now, because a
missing one is a host's business — and the MCP server learned to say so.

---

## 48. The list, checked: what this kit claims and does not claim

A tree that keeps saying what it has not built is a tree whose claims
can be believed. So here is every claim this README has made about what
is **not** there, checked against the tree, with the section that says
so.

| the claim | the truth today | where |
|---|---|---|
| a **numeric core**: a typed length, a typed angle | **built**, and **not in the core.** A `Length` in millimetres and an `Angle` in radians are a **library** and a **plugin** (§52), so every number in a document is still a JSON number and the core still has no unit — which is the answer §39 gave | §39, §43, §52 |
| a **typed parameter**: a command that takes a length, and the schema says so | **built.** `ParamKind` was never sealed, so a library could always implement it for its own type; what was missing was the schema's **name and unit**, and that is the constrained change §56 made | §52, §56 |
| the **panic guard** | **built**, twice: the command path and the evaluation path, and the plan says what it could not say | §38 |
| the **geometry** | **built**: a library of its own, with no commands and no document knowledge | §13, §43 |
| the **file format**: the old tree's `.craftkit` | **by design, untouched.** v2's format is a contribution, with migrations that are contributions, and a version chain the ecosystem checks | §15 |
| a **toolbar** | **not built.** v2 has no UI toolkit and no window | §16 |
| a **control channel** | **built**: the session behind a token-authenticated socket, with the family's wire format and the family's methods — and no `ui.*`, because this tree has no window | §55 |
| the **MCP server** | **built**: one tool per command, generated from the registry, with the tool list being the command list | §41 |
| a **real document model**: layers, text, images, an appearance of its own | **built.** Layers are nodes and edges (§44); text is an element in two batteries (§33, §44), a paragraph's own text and runs in a third (§49), and a paragraph **broken into lines** in a fourth (§58) — with the widths coming from the caller and the **letters** from the kit's own typeface (§128, §129); **images are ported** (§142) and the **appearance** is the kit's own (§95) | §44, §49, §58, §95, §142 |
| a **selection of spans** | **built** | §22 |
| the **view state** | **built**, and it is the session's | §37 |
| the **bundle** the application writes, and the bundle a plugin fills | **built**, both | §25.1, §33 |
| the **plugin formats** (CLAP, VST3, AU) | **the door is built**, and **all three formats have come through it**: CLAP (§76–§87), VST3 (§116–§120) and AU (§121–§123), each with its own host. What AU has not brought is its **battery**: its parameters and its state are CoreFoundation's | §34, §35 |
| the **history** that branches | **built** | §24 |
| a **task** that may outlive the process | **built** | §28 |
| one **camera** with its own caps | **built** | §29 |
| the **prompt**: a host that has what a command is missing | **built**, both halves: the host's turn before the command runs (§45), and a command that asks **while it runs** (§50) | §45, §50 |
| **copy-on-write**, so the undo is cheap | **built**: the nodes are shared and the states keep what they were, and the cost of an edit no longer depends on the size of the document — measured, before and after (§53) | §40, §43, §53 |
| a **typed relation to a part** of a thing (an area, a sub-part) | **not built, and not needed**: two ports needed it and both kept it in the payload, where the plugin owns it | §42, §43 |
| the **reader's** keeping of what it does not understand | **built**, at the document and at the node | §33, §43 |
| the **plan**: one evaluated thing every consumer reads | **built** | §24 |
| the **guard** around evaluation | **built** | §38 |
| a **shared library** a program opens and a plugin in it answers | **built**, and it is a command | §35, §36 |
| the **version a file is written at**, derived from what the document holds | **built.** A file is written at the oldest version that loses nothing, and the version is **tried** rather than declared: each version is written and read back, and the one that keeps everything wins. The round trip that decides it found a bug — a relation's payload was lost from every file (§57) | §51, §57 |
| a **typeset** paragraph: a line breaker and the widths it needs | **built, and the widths are the caller's.** The breaker is a **library** (§58) and the paragraph is a **battery**, and the kit now has **its own typeface** (§128): the widths live in a **metric node** — advances, runs and pairs (§60) — the paragraph's typeface is an **edge**, and the caller says what the widths are (§58, §59, §60). The paragraph is **drawn** with those letters (§129) | §58, §59, §60, §128, §129 |
| **hyphenation**: where a word may be broken | **built.** The breaker knows a **hyphenation point** — a flagged penalty — and the limit on how many may follow each other (§58); and now the kit **finds** them: the patterns say where the words of the language break, the heuristic answers for the words the patterns do not know, and the limits say how careful the answer is (§145) | §58, §145 |
| a **caret**: where the reader is in the text | **built.** The caret is a **node** under the paragraph and it carries the character it stands before; the text is the words with a space between them, and the space is a character too; the caret's **place** is derived from the layout and not stored (§151) | §151 |
| a **selection of characters** | **built.** The core asks a kind where the picked-out **part** of its node is, and the answer may be more than one piece; a paragraph's own unit is the character, so the SDK's span selection picks out characters and the paragraph says where they are (§152) | §22, §152 |
| an **edit that replaces what is picked out** | **built.** Typing writes over the picked-out range and deleting takes it away; the caret stands after what was typed (§153) | §153 |
| a **clip drawn as its sound** | **built.** The **peaks** of a source are measured once and kept in the document, and a clip's bar carries the shape of the part of the source it plays (§154) | §154 |
| a **better resampler** | **built.** The resampler is a **windowed sinc** — an ideal low-pass with a Kaiser window — so what was above the new half of the rate is taken away instead of folding back as a whistle; a sound too long for the limits is refused (§155) | §137, §155 |
| a **shape that follows the automation** | **built.** The gain of a clip's sound at a time is the clip's own gain and the track's curve, and the shape on the bar is drawn with it (§156) | §156 |
| a **caret that moves a word and a line at a time** | **built.** The word is in the words and the line is in the layout, and the caret keeps its place across a line (§157) | §151, §157 |
| a **resampler with a bank** | **built.** The kernel is precomputed for every phase of a rational rate change, so an output is a dot product; a rate change with no bank worth having is read tap by tap (§158) | §155, §158 |
| a **band for each channel** | **built.** The peaks are the peaks of each channel and the bar carries a band for each of them; the buckets and the channels share the table (§159) | §154, §159 |
| a **send that is panned and muted** | **built.** A send's copy is panned on its way to the bus, and a muted send is a copy of nothing; a pan that is not a pan is refused (§160) | §160 |
| a **pre-fader send** | **built.** The track's stage is the clips, the inserts, the mute and then the fader, and a send may take its copy from before the fader; a muted track is not heard anywhere (§161) | §160, §161 |
| a **master with a fader** | **built.** A track whose kind is `master` holds the fader of the whole mix: its inserts are run over the mix, its lanes move it, and the metronome is summed after it (§162) | §162 |
| a **bus that is read** | **built.** A track may read a bus, and a bus that is read is heard through the reader and not by itself; a reader holds no clips, has no sends and is not routed (§163) | §163 |
| a **route that is honoured** | **built.** A track that feeds a bus is heard on that bus and not in the main mix, and a bus that feeds another bus is heard there; the buses are fed in the order they are read in (§164) | §164 |
| a **track with a trim** | **built.** The trim is the level of the track's input and it comes before the inserts; the phase is a sign and not a level, and a trim outside the tree's range is refused (§165) | §165 |
| a **clip with effects of its own** | **built.** An insert is on one thing — a track or a clip — and a clip's chain runs over the clip's own sound, after the clip's gain and before the track's inserts (§166) | §166 |
| a **track with takes** | **built.** A track may have several takes and one of them is the one that is played; the take is named, and a track whose active take is gone plays its first (§167) | §167 |
| a **muted clip** | **built.** A clip may be muted: the render skips it and the clip stays in the document, so a quiet clip is a clip that may be loud again (§168) | §168 |
| a **path that may be edited** | **built.** A point of a path may be moved and the handles of its curves set; the handles move with the point, and a handle on a straight segment makes it a curve (§169) | §169 |
| a **path that may be painted** | **built.** The fill and the stroke of a path may be set: what is inside it and what the line on its outline is; a colour that is not a colour is refused (§170) | §170 |
| a **path that says what is inside it** | **built.** The fill rule of a path may be set, and two paths may be excluded — the difference the other way round, which is in the library and was not in a command (§171) | §171 |
| a **PDF stream's filter** | **built.** The five ways a stream's bytes may be encoded are undone — Flate, LZW with the early change, ASCIIHex, ASCII85 and RunLength — and the filter may be named in the command or in the dictionary (§172) | §172 |
| a **PDF stream's predictor** | **built.** The step that was made after the encoding is undone: the TIFF predictor and the five PNG ones, with the width of the picture from the dictionary (§173) | §173 |
| a **line's corners and ends** | **built.** The stroke carries a join and a cap, the rasteriser draws them, and two commands say which (§174) | §174 |
| a **page tree** | **built.** A page may have a parent, and the four attributes a page inherits are found by walking up the tree (§175) | §175 |
| a **content stream** | **built.** The operators of a page's content are read, with the operands each one was given (§176) | §176 |
| a **path that is placed** | **built.** A path may carry a transform — the six numbers of a matrix — and it is drawn, hit and measured where the transform puts it (§177) | §177 |
| an **object stream** | **built.** The objects a file keeps in one place are read: each one's number and what it says (§178) | §178 |
| a **cross-reference table** | **built.** The file's own index of itself is read: the classic table and the cross-reference stream (§179) | §179 |
| a **file loaded into the document** | **built.** The objects of a file become nodes, its references become relations, and the bytes of its streams go into files of their own (§180) | §180 |
| a **reader that feeds a bus** | **built.** The buses and the readers are mixed in the order their routes say, and a mix that loops is refused (§181) | §181 |
| the **outline of a line** | **built.** The shape a stroke covers is a path of its own, which may be filled, measured and cut (§182) | §182 |
| a **file written out** | **built.** The document is written out as a PDF file, and the file that comes out is read back (§183) | §183 |
| a **smooth point** | **built.** A point's two handles become one: collinear and of the same length, and a straight side becomes a curve (§184) | §184 |
| the **spacing** of the clips | **built.** The clips are butted against each other, with a gap between them, end to start or start to start (§185) | §185 |
| the **loop** of a clip | **built.** The copies of a clip are made, and each one is named by the clip it follows — so the chain is a walk and the loop is not inferred from the track (§186) | §186 |
| a **clip that snaps** to its neighbour | **built.** Next and Previous, the clip nearest the destination first, and the clips that are named are the destinations too (§187) | §187 |
| a **PDF annotation** | **built.** An annotation is an object of its own, the page names it in its `/Annots`, and the action of a link is an object as well (§188) | §188 |
| a **dictionary** inside a dictionary | **built.** The payload holds one as a list of pairs under `entries`, and the writer writes it back as a dictionary (§189) | §189 |
| a **gradient** | **built.** The fill is a paint — one colour or a gradient — and the gradient is the kit's own (§190) | §190 |
| a **blend mode** | **built.** The sixteen modes of the PDF and the screen, and the kit says how each of them mixes (§191) | §191 |
| the **copies** of a shape | **built.** A grid, a mirror over a line and a ring round a centre — the copies are paths of their own, and the transform says where each one is (§192) | §192 |
| the **order** of the drawing | **built.** The order of the tree is the order of the drawing, and two commands say it — the kit's own lookup now reaches a node wherever it is (§193) | §193 |
| a **reference in the dictionary** | **built.** A reference is an edge and a value at once, the page tree says itself in the file, and the count of the children is derived from the edges (§194) | §194 |
| the **outline** of a document | **built.** The bookmarks are a tree of items with titles and destinations, the list of siblings is relinked, and the counts are derived from the visible items (§195) | §195 |
| the **beat** of a clip | **built.** The place inside a clip where the beat falls, an option and not a zero — and it follows the audio when the clip is trimmed (§196) | §196 |
| the **locks** on a clip | **built.** The content lock and the place lock are two locks, and the commands that would break one of them refuse (§197) | §197 |
| the **marks** of a clip | **built.** The rating of stars is the clip's own, and the colour is the kit's appearance — so the battery has no second way to say it (§198) | §198 |
| the **group** of a clip | **built.** A group is a node with a name and the clips point at it, so the name is the handle and the membership is an edge (§199) | §199 |
| the **grid** of the session | **built.** A division of the bar, counted from the bar line, and a clip's beat is put on the nearest line (§200) | §200 |
| the **gain** of a clip | **built.** The level of the clip itself, refused outside its range, and a bypass that keeps the gain instead of throwing it away (§201) | §201 |
| the **conform** of a clip | **built.** The material is played at the rate that makes it fit the tempo, the tempo of the material is the clip's own, and the beat follows the audio (§202) | §202 |
| the **name** of a node | **built.** A node has a name and nothing could change it; one command renames any node (§203) | §203 |
| the **copy** of a node | **built.** A node and everything under it, with the edges copied and the copies joined to the copies (§204) | §204 |
| the **group** of tracks | **built.** A group of tracks is a link: the members are mixed together, and a group that is off links nothing (§205) | §205 |
| the **elastic** of a track | **built.** An algorithm on the track says that its clips are stretched and the pitch stays, and the bound is the tree's own (§206) | §206 |
| the **level** of a track | **built.** The trim is the input and the fader is the output, the fader is the volume lane's default, and the group links it (§207) | §207 |
| the **pitch** of a clip | **built.** A number of semitones on the clip, refused outside its range, and the length is the same (§208) | §208 |
| the **processes** on a clip | **built.** A name and its parameters, done in order, refused when the name is not one this battery knows (§209) | §209 |
| the **pan** of a track | **built.** A balance from minus one to one, the default of the pan lane, applied when the track is routed (§210) | §210 |
| the **stems** of a session | **built.** The mix with one track in it, one file for each track, written into a directory (§211) | §211 |
| the **separation** of a clip | **built.** One clip becomes two at a place inside it, and the two are the same sound (§212) | §212 |
| the **mute** of a track | **built.** A switch on the track, and the mute lane is its default (§213) | §213 |
| the **solo-safe** track | **built.** A track that a solo does not silence, and a command that says so (§214) | §214 |
| the **separation** on the grid | **built.** A clip cut at every line of the session's grid it crosses, refused past the bound (§215) | §215 |
| the **healing** of a track | **built.** The clips that were separated are joined again when they are the same sound continued (§220) | §220 |
| the **separation** at the transients | **built.** A clip cut where its sound starts again, found in the spectrum as it is in the tree (§221, §244) | §221, §244 |
| the **stripping** of silence | **built.** The quiet parts of a clip are thrown away and the loud parts are kept and named (§222) | §222 |
| the **auto rename** of a clip | **built.** A scheme is a node and the clips point at it, and the piece after a cut takes the scheme's next name (§216) | §216 |
| the **events** of the session | **built.** A marker is moved and renamed, the tempos are scaled and a range of the map is made constant (§228) | §228 |
| the **controllers** of a MIDI clip | **built.** The volume, the pan, the bend and the program may be changed, and the duplicate notes taken out (§227) | §227 |
| the **moving** of a clip | **built.** A clip moves along the timeline with its content, and a locked clip does not move (§226) | §226 |
| the **editing** of a lane | **built.** The breakpoints of a curve are set, taken away, cleared, written and thinned (§225) | §225 |
| the **taking away** of a link | **built.** A route, a reading, a send and a source can all be taken away, and a link that was never made is refused (§224) | §224 |
| the **MIDI** of a document | **built.** A clip may hold notes: the content is a field in the payload, the notes are read from and written to a Standard MIDI File, and they may be quantised, transposed, given a velocity, and added, changed, taken away and moved (§217, §218, §219, §223) | §223 |
| the **velocity** of a note | **built.** The notes are struck at a velocity, changed by one, scaled, limited and ramped, and a velocity outside the range is refused (§229) | §229 |
| the **length** of a note | **built.** The notes are given a length, changed by one, scaled and made legato, and a note is never shorter than a tick (§230) | §230 |
| the **modes** of the strip of silence | **built.** The silence is stripped, the sound extracted or the clip separated, and the loud parts can be read without changing anything (§231) | §231 |
| the **cut** of a MIDI clip | **built.** The cut divides the notes, and each piece keeps the notes that are its own (§232) | §232 |
| the **trim** of a MIDI clip | **built.** The trim takes the notes off the head and counts the rest from the clip's new head (§233) | §233 |
| the **automation** of a whole track | **built.** Every lane of a track is written with the static value of its parameter or thinned at once (§234) | §234 |
| the **fade** of a clip to a place | **built.** A clip is faded up to a place or down from it, and the shape is the tree's own (§235) | §235 |
| the **time** of the session | **built.** Silence is inserted at a place and time is cut out of a range, and everything after moves (§236) | §236 |
| the **names** of the notes | **built.** A pitch, a drum and a program are named by the tree's own tables, and a clip's notes are read with their names (§237) | §237 |
| the **key** of the session | **built.** The key is set at a place and read back, kept as the sharps and the mode a file carries (§238) | §238 |
| the **music** of a MIDI file | **built.** The file's tempos, meters and keys come in with it and go out with it, and the notes land where the music lands (§239) | §239 |
| the **tracks** of a MIDI file | **built.** Every track of the file becomes a track of the session, with a clip of its own (§240) | §240 |
| the **consolidate** of a track | **built.** The sound of a track's clips over a range is written to a file, and the clips become one clip of it (§242) | §242 |
| the **taking away** of a node | **built.** A clip, a marker and a track can be taken away, and what goes with them goes with them (§243) | §243 |
| the **onsets** in the spectrum | **built.** The tree's own detector, with the transform written here because the kit has no dependencies (§244) | §244 |
| the **split** into mono | **built.** A stereo track becomes a track for each channel, panned to the sides, and a mono source is heard in every channel of the mix (§245) | §245 |
| the **repeating** of a range | **built.** The material of a range is copied and laid after it, as many times as asked (§246) | §246 |
| the **clipboard** | **built.** The material of a range is copied, cut and pasted, and the clipboard is in the document (§247) | §247 |
| the **voice** of a track | **built.** A track's voice is turned off and on again, and an inactive track is not heard (§248) | §248 |
| the **commit** of a track | **built.** The sound of a track is written to a file, a new track of it is made, and the old track goes quiet (§249) | §249 |
| the **chain** of inserts | **built.** An insert is taken away and it moves in the chain, and the chain is the order of the nodes (§250) | §250 |
| the **automation** of a range | **built.** The clipboard carries the breakpoints of the range, the paste writes them and the cut takes them away (§251) | §251 |
| the **name** of a thing | **built.** A track and a clip are renamed, and a name that is empty or too long is refused (§252) | §252 |
| the **settings** of a send | **built.** The level, the pan, the silence and the place of a send are changed (§253) | §253 |
| the **duplicate** of a track | **built.** A track is copied with its clips, its automation, its inserts and its sends (§254) | §254 |
| the **render** of a clip | **built.** The sound of a clip is written to a file, and a MIDI clip is refused (§255) | §255 |
| the **taking away** of an event | **built.** A tempo, a meter and a key go away, and the document's own stays (§256) | §256 |
| the **shapes** of the tree | **built.** A star, a line and a spiral are made out of numbers, with the tree's own geometry (§257) | §257 |
| the **turning** of a path | **built.** A path's open contour is closed and a path is turned the other way round (§258) | §258 |
| the **anchor** of a segment | **built.** A segment is split in two, and the two halves are the same curve (§259) | §259 |
| the **key** of an object | **built.** A key of a PDF object's dictionary is set and taken away, and the order is kept (§260) | §260 |
| the **taking away** of an object | **built.** An object of a PDF file is taken away, and the references that named it go with it (§261) | §261 |
| the **number** of an object | **built.** The number of a PDF object is changed and every reference to it follows, and no two objects share one (§262) | §262 |
| the **trimming** of a clip | **built.** The head or the tail of a clip is trimmed, and a trim that would cross a bound is refused and says which bound (§263) | §263 |
| the **moving** of a clip | **built.** A clip moves along its track or to another one, and a move that would land on another clip is refused (§264) | §264 |
| the **keyframes** of a property | **built.** A keyframe is set and taken away, and the value of a property at a frame is the line between the keyframes (§265) | §265 |
| the **inserting** of a clip | **built.** A gap opens at the place and the clips that are there are pushed along, so that nothing is cut (§266) | §266 |
| the **splitting** of a clip | **built.** A clip is divided at a place, the second piece is a new clip, and the media moves with the cut (§267) | §267 |
| the **taking away** of a clip | **built.** A clip is taken off its track, the edge that says which item it played goes with it, and the gap is a gap (§268) | §268 |
| the **rippling** of a clip | **built.** A clip is taken away and the gap closes, and the ripple is refused when the time the clip took is not free (§269) | §269 |
| the **filling** of a range | **built.** A range is filled from a cell — copied or counted on — and the target holds the source at a corner (§270) | §270 |
| the **modes** of the fill | **built.** The tree's four fills are here — copy, series, extend and alternate — and the source may be a range (§274) | §274 |
| the **number format** of a cell | **built.** A cell carries a number format and the text it shows is derived from its value and that format (§271) | §271 |
| the **lifting** of a range | **built.** The clips in a range of a track are taken away and the gap they leave is a gap (§272) | §272 |
| the **extracting** of a range | **built.** The clips in a range of a track are taken away and the gap closes (§273) | §273 |
| the **filling of a formula** | **built.** A fill copies a formula and shifts its references, and an anchored reference does not move (§275) | §275 |
| the **date** of a cell | **built.** A date is a number and a date format, and the tree's calendar has the day in it that never happened (§276) | §276 |
| the **fill of dates** | **built.** The tree's four date fills — days, weekdays, months and years — step through the calendar (§277) | §277 |
| the **`TEXT`** of a formula | **built.** A formula writes a value with a format code, and the code may be in a cell (§278) | §278 |

### The list of things that are honestly not there: empty

It is empty, and that is not the end of the work — it is the end of
*this* list. The table above still has rows that are not built (the
toolbar, by design; the plugin formats, which are a shim over a door
that is built, and the AU's own battery, which is CoreFoundation's),
and those are **not** on this list because they are not gaps in the
kit's design: they are things this tree has not been asked for. (The
images were ported in §142, and the sentence that said they were not
has been here since before that, and was wrong.)

What was on this list, and where each of them went:

- the **resumable command** — §50, and §50 says what the answer costs:
  the command is **run again** when the answer comes, so a command that
  asks must ask **before** it changes the document, and the session
  checks that rather than trusting the command.
- the **cheap undo** — §53: the nodes are shared, the states keep what
  they were, and the cost of an edit no longer depends on the size of
  the document — measured before and after.
- the **typed parameter** — §56, and §56 says that the claim which put
  it on this list was wrong: the seam was open, and what was missing
  was the schema's name and its unit.
- the **version a file is written at** — §57: a file is written at the
  oldest version that loses nothing, and the round trip that decides it
  found a bug in the format.

Three items used to be on this list and are not now. The **resumable
command** — a command that asks while it runs — was built in §50, and
§50 says what the answer costs: the command is **run again** when the
answer comes, so a command that asks must ask **before** it changes the
document, and the session checks that rather than trusting the command.
The **cheap undo** was built in §53: the nodes are shared, the states
keep what they were, and the cost of an edit no longer depends on the
size of the document — measured before and after. The **typed
parameter** was built in §56, and §56 says that the claim which put it
on this list was wrong: the seam was open, and what was missing was the
schema's name and its unit.

### And the one that is not a code gap

The **code mutation** is CI-only by the machine's rule, so no claim in
this README has been measured by it. The v2 gates are: the tests,
clippy, fmt, the workspace-scanning gate that holds the list of allowed
`unsafe`, and — since §54 — the tree's **own** gate, which holds the
deny attribute, the layering and what a plugin may depend on, and is
measured by being pointed at trees that are broken on purpose.

The outer tree's gates do not reach into `v2/`, which is excluded from
the outer workspace — so the outer gate is evidence about the outer tree
and **not** about this one. §54 is.

---

## 49. The ninth battery: a word processor, and the rule that has a limit

§40 surveyed WordCraft: 632 commands, one `Session::run`, a panic
guard, and a document that is **a flat list of blocks, a side vector of
styles, and a table of media**. It is the family's largest tree, and it
is a word processor — so it is the ninth battery:
`craftkit-plugin-prose`.

### What was ported

| the word processor | v2 |
|---|---|
| the document | a **node** of the kind `prose.document` |
| a block | a **node** of an **open kind**: `prose.block.paragraph`, `prose.block.table` |
| a paragraph's text | the block's **payload**, stored **once** |
| a paragraph's runs | the **payload**: a run is an **interval** over the text |
| the words a run covers | **derived**, every time, by the evaluator |
| a style | a **node** — few things are nodes |
| a block's style | a **relation**: an edge that says something |
| a style's base | a **relation**: an edge that says something |
| a table's cells | the **payload**: a cell holds a value and not a tree |
| a media | a **node** whose **stream** is the file |
| the blocks | **derived**, in order, by the evaluator |

Seven commands, five kinds, one evaluator, one panel, **24 tests**.

### The five things the port found

- **A flat list is a shape the tree already had.** The blocks are a
  sequence and not a nesting, so the tree is a list of children and
  **the order is the tree's order** — no index field, no key. That is
  the third shape a document has had here — a grid (§42), a flat store
  (§44) and a list — and the tree took the first and the third without
  a word being said.
- **A run is an interval, and the words it covers are derived.** The
  application stores a paragraph's text **once**, as a string, and a
  run is a **length** over that string plus the formatting. The run
  does not carry the words: it says how many of them it covers. Here
  the run is an interval in the payload and the **words it covers are
  worked out every time**. That is the **fourth** instance of "derived
  values are derived" (after the timeline's lengths, the artist's
  element durations and the CAD's extents) and the first where the
  derived thing is **text**.
- **The rule "what is numerous is in the payload" has a limit, and this
  port found it.** A table cell in the application holds a **nested
  list of blocks** — and a thing that holds a tree cannot be a value
  in a payload. So the rule is not that, but the sharper one:
  **what is numerous and holds nothing is in the payload; what holds a
  tree is a node.** The grid's cells hold numbers, the CAD's geometry
  holds numbers, a paragraph's runs hold formatting — all payload. A
  cell that holds paragraphs is not, and this battery says so rather
  than pretending: its table's cells hold values, and the port of the
  nesting is **not built**.
- **A style is a node, and a paragraph's style is an edge.** The third
  time an edge has answered "what is this relationship?" — after the
  PDF's reference (§27) and the CAD's layer (§44) — and the third time
  the answer has been the same. A style's **base** is an edge too, and
  the chain of styles is **derived** by walking the edges — bounded, and
  stopped at a style it has already seen, so a cycle is not a hang.
- **The media is a stream, and the port landed on a seam that was
  already there.** The application's media table is a map from a key to
  a file; here a media item is a node whose **stream** is that file
  (§25), and the bundle carries the bytes without anybody naming a
  path twice. Nothing in the core moved.

### What the port does not carry

Named, so that they are not mistaken for things that are there:
**sections** (page size, margins, columns), **headers and footers**
(the application's *parts*, which are separate stories), **track
changes** (revisions, which the application stores as indices into a
side table), **numbering**, **comments**, **content controls**, **fields**,
the **layout engine** (line breaking, pagination, fonts) and the
**copy-on-write** document that makes the undo cheap (this port
confirmed the family's way of doing it — the blocks are shared by `Arc`
— and **§53** took it).

The missing parts are **not** missing because the kit could not hold
them. They are missing because the battery is the **document model**,
and the rest of that application is a layout engine — which is a
library, and §39 says where a numeric core goes when it arrives.

### And the cost

**Nothing.** No field, no method, no version, no change in the core.
Seven commands, five kinds, one evaluator, one panel, and the
twenty-fourth test through the twenty-fourth.

The pattern is now nine ports old and **seven** of them cost nothing;
the two that cost — the artist's closed unions (§33) and the branching
history (§24) — bought the kit's two strangest features. Every domain
the family has since shown this kit has fitted **the tree it already
had**, and the fittings are the interesting part: a grid is a payload,
a drawing is a flat store, a word processor is a list, and a run is an
interval.

### The table, with the ninth row

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
| the **word processor** | a run is an **interval**; the words it covers are **derived**; a cell that holds a tree is not a payload | nothing |

---

## 50. The resumable command: a command that asks while it runs

§45 built the **first half** of the prompt: the host's turn *before* the
command runs. §44 said what was missing, in the words of the survey: a
command that asks *while it runs* — CADCraft's `interactive:
Option<Factory>`, a command that draws a preview, waits for a point, and
keeps going until the person says Enter. §48 listed that as one of the
**three** things this kit did not have.

This section is the third thing answered.

### What was built

- **`Session::ask(&Question)`** — the command asks. It is one method on
  the session, and a command calls it while it runs.
- **`EngineError::NeedsInput`** — the signal that travels out of the
  command. The `?` is the whole of it.
- **`Outcome::needs_input`** — what the host is told: the question, and
  the fact that nothing has changed.
- **`Session::answer(value)`** — the host answers, the command runs
  again, and this time the answer is waiting for it.
- **`Session::cancel()`** — giving up, which costs nothing.
- **`Session::pending()`** — what is waiting, if anything.
- **`doc.ask` and `core.note`** — the worked example: a command that
  asks, and a node of the core's kind that holds the question and the
  answer in its payload.
- **The MCP door** — the agent is told what is wanted **in the same
  words as a command that is missing a parameter**, and calls the tool
  again with the answer in it. The agent learns one thing and not two.

### The rule, and why there is one

> **A command that asks must ask before it changes the document.**

The rule is enforced by the **session**, not by a convention: the
session compares the document with the snapshot it took before the run,
and a command that changed something and *then* asked is **refused and
put back**.

The reason is the reason the whole design turns on. Rust cannot suspend
a call and pick it up where it left off — there is no generator in the
language, and this kit does not have one. So the answer to "a command
that asks while it runs" is: **the command runs again**, and the answer
is waiting for it. And running a command again is only safe if it
changed nothing — so that is the rule, and the session checks it rather
than trusting the command.

This is the **constrained** answer, and it is a real difference from the
CAD case, so it is written down rather than glossed over: CADCraft's
interactive commands keep a **live** preview and a live prompt, drawing
as they go. Here the command may **compute** and **preview** — that is
not the document — but it may not **change the document** before it
asks. So the live part is the preview, and the preview is the host's.

### The bounds, and why they are here

- **Thirty-two questions** for one command. A command that asks for
  ever is a command that never finishes, and the host is the one that
  has to notice; the kit notices it instead.
- **Thirty-two answers** for one command. The same bound from the other
  side, because a command and a host that keep answering each other is
  a loop.
- **A pending command has changed nothing**, so dropping it costs
  nothing — and that is what makes the third rule safe: another command
  may run while one is waiting, and the waiting one is dropped. The
  document is exactly as it was.

### What it cost

Almost nothing, and the shape of the cost is the point:

- `Outcome` gained **one field** — `needs_input`, a question or
  nothing;
- `EngineError` gained **one variant**;
- the **document** gained nothing: no field, no version, no node kind
  beyond the plain `core.note` the example needs;
- the **registry** gained nothing: no command gained a "prompts" field,
  and no kind became interactive;
- the **command's own shape** did not change. It is still a struct with
  typed fields, and asking is a call it makes while it runs.

### What it does not settle

- **The live preview.** A command may compute what it would draw, but
  the kit has no canvas to draw it on — there is no UI toolkit and no
  window (§16) — and no place in the document to put a preview. The
  preview is the host's, and the seam gives the host the question and
  nothing else.
- **The person at a prompt.** The kit has no UI, so the host is a
  program: an agent, a script, a test. The three are the same seam
  (§45), and a person would be a fourth host behind the same door.
- **The waiting command is not in the document.** It is the session's
  own state, the way the selection and the camera are. A document saved
  while a command is waiting has no trace of the question, and that is
  a thing to decide rather than to inherit: the command was waiting,
  not the document.

---

## 51. The tenth battery: a page layout document, and the edges that were id fields

§40 surveyed DesignCraft: 542 commands, one `Session::execute`, and a
document that is **not a tree and not a graph** — a set of **spreads**,
each with **pages** and a flat, z-ordered list of **items**, beside
**layers**, **stories**, **styles** and an **asset table**. It is the
family's publishing application and the last of its trees that had not
been ported, so it is the tenth battery: `craftkit-plugin-layout`.

### What was ported

| the page layout application | v2 |
|---|---|
| the document | a **node** of the kind `layout.document` |
| a spread | a **node** of the kind `layout.spread` — pages and items |
| a page | a **node** of the kind `layout.page` |
| an item | a **node** of an **open kind**, and the **caller names it** |
| a layer | a **node** — few things are nodes |
| a story | a **node**: the text, stored once |
| an asset | a **node** whose **stream** is the file |
| an item's layer | a **relation**: an edge that says something |
| an item's story | a **relation**: an edge that says something |
| a story's frames | **derived**: the inverse of the edges |
| the pages of a spread | **derived**, in order, by the evaluator |

Nine commands, ten kinds, two evaluators, one panel, **14 tests**.

### The five things the port found

- **The typed id fields are the edges.** The application writes
  `Item.story`, `Page.parent`, `Story.frames` and `Item.overrides` as
  hand-written id fields, and the survey says a universal kit would
  "type them once". Here they are **relations**: an edge with a payload
  and a kind that says what it is. That is the **fourth** time an edge
  has answered "what is this relationship?" — after the PDF's reference
  (§27), the CAD's layer (§44) and the prose's style (§49) — and the
  fourth time the answer has been the same.
- **A relation has an inverse, and the inverse is derived.** The
  application stores `Story.frames` as a field of its own, so the two
  ends of the same fact are stored **twice** and can disagree. Here the
  edge is stored **once**, from the item to the story, and the story's
  frames are **worked out** from the edges. That is a new kind of
  derived value in this kit: not a number, not a geometry and not a
  timeline, but a **relation**.
- **A closed union of contents is exactly what the open kind is for.**
  The application's item content is `Unassigned | Text | Graphic |
  Group`, matched exhaustively in the document, the composer, the
  renderer and every exporter — so a new kind of content is a change in
  five places at once. Here the **caller names the kind**, and the list
  is open. The fifth port to find this, after the artist's unions and
  the CAD's entities.
- **The asset's bytes are in the package, not in the document.** The
  application keeps them beside the document and the document holds the
  id. That is this kit's **stream** (§25) — the **third** port to land
  on it, after the artist's stage and the prose's media — and the
  bundle carries the bytes without anybody naming a path twice.
- **A story is a string, and what formats it is said over the text.**
  The second tree to say it, after the word processor: the text is
  stored **once**, and the runs are intervals over it. Two trees, one
  shape, and the shape is the kit's.

### And the finding that is not a document model

The application's file format writes the **oldest version that still
represents the document** — `required_version(doc)` returns 4 for a
document with graphic layer options, 3 for one with EPS assets, and 2
otherwise — so that old readers keep working. This kit writes the
**current** version and has a chain of migrations to read the old ones.

That is a difference worth naming, and it is **not built here**: a file
written by this kit always says the newest version, even when the
newest version says more than the document needs. What the application
does is a **downgrade on write**, and the kit's version chain is the
machinery it would need. It is written down rather than built, because
"the version a file is written at" is a decision about the format's
contract, and that decision is the owner's.

### The table, with the tenth row

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
| the **word processor** | a run is an **interval**; the words it covers are **derived**; a cell that holds a tree is not a payload | nothing |
| the **page layout application** | the typed id fields are **edges**; a relation's **inverse is derived**; the file's version is **derived from what is in it** | nothing |

Ten ports. Seven of them cost nothing. The two that did — the artist's
unions and the branches — bought the kit's two strangest features, and
everything the family has taught it since has cost **nothing**.

---

## 52. The typed value arrives: a library and a plugin, and not a core change

§39 asked what a typed value would look like when it arrived, and the
answer was that it arrives as a **plugin** and a **library**, and not as
a change to the core. The CAD port confirmed it (§44): the geometry is
in the payload in the world's own units, and the evaluation that does
arithmetic reaches for a **library** — not for a unit in the core.

This section builds that answer.

### What was built

- **`crates/craftkit-numeric`** — a **library**: a `Length` in
  **millimetres** and an `Angle` in **radians**, and the arithmetic over
  them. Both are newtypes over `f64`, so a millimetre cannot be passed
  where a radian is wanted: the mistake is a compile error and not a
  number that is wrong by a factor of fifty-seven. Fourteen tests.
- **`crates/craftkit-plugin-numeric`** — a **plugin**: the kinds of
  thing that have a length or an angle, the commands that make them, and
  the evaluation that says what they measure. Five commands, three
  kinds, one evaluator, one panel, fourteen tests.

### What it proves

- **The core has no unit, and it never will.** A length in a document
  is a **number** in a payload, and the number is millimetres because
  *the plugin* says so. The core reads the payload as JSON and knows
  nothing about it: there is no "millimetre" anywhere in the SDK, and
  there is no place to put one that would not be a plugin's business.
- **The typed value is not a command parameter.** The kinds a parameter
  may take are the **core's** — text, number, bool, a node reference —
  and a plugin cannot add one. So a command takes a **number and a unit
  name**, and the conversion happens in the command's body, in the
  library. That is the one thing that would need the core to change, and
  it is **named here** rather than worked around silently: a typed
  parameter is a core change, and this kit did not make one.
  — **This bullet was wrong, and §56 says so.** The trait is the SDK's
  and it is **not sealed**, so a library could implement it for its own
  type from the day it was written; the derive has always been generic
  over it. What was missing was the *schema's* name and unit, and that
  is what §56 added.
- **The other units are derived.** A length is stored once, in
  millimetres, and the centimetres, the metres, the inches and the
  points are worked out every time by the evaluation. The same is true of
  an angle's degrees, and of a box's area — which is the library's
  arithmetic and not the core's.
- **The library is a library.** The application names the **plugin** and
  not the library, so the linker does not wire the library in, and the
  library contributes nothing to the registry. A test says so, by
  reading the application's own manifest.

### The hostile input, once more

A length that comes from outside is not a length until it has been
**checked**, and the check is in the library rather than in the command:
a number that is not finite is refused, a number outside the range the
model holds is refused, a unit that is not a unit is refused, and a
list of a hundred thousand lengths is refused. The commands say so in
their own words, and the tests say so in theirs.

### What it cost

- a **library** with two types and their arithmetic;
- a **plugin** with five commands and three kinds;
- **nothing in the core.** No field, no variant, no method, no version.

### And what it does not settle

The **typed parameter** is the thing this arc found missing, and it is
not built: a command that takes a length takes a number and a unit
name, and the conversion is in the body. That is a real cost — an agent
reading the schema sees `{"type": "number"}` and a unit name beside it,
and not `{"type": "length"}` — and it is a **core** question, not a
plugin's. The kit's own rule is that a plugin may not change the core,
and this arc did not.

It is written down rather than decided, because the decision is the
owner's: the kinds a parameter may take are the core's list, and adding
one is a change to every parameter's schema, every front end's reader
and every agent's idea of what a number can be.

— **§56 answered it**, and the answer was partly that this section was
wrong: the *reading* was never the problem, because `ParamKind` is open.
The *schema* was, and the change §56 made is the constrained one: two
optional constants and two optional fields, so that a kind can say
**what it is** and **what unit it is in** without the core's list
growing.

### The table, with the eighteenth row

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
| the **word processor** | a run is an **interval**; the words it covers are **derived**; a cell that holds a tree is not a payload | nothing |
| the **page layout application** | the typed id fields are **edges**; a relation's **inverse is derived**; the file's version is **derived from what is in it** | nothing |
| the **typed value** | the unit is a **plugin's** business; the typed value is **not a command parameter** | a library and a plugin |
| the **line breaker** | the widths come from the caller and the **letters** from the kit's own typeface (§128); the measure is the paragraph's **place**; the lines are **derived**, and the derivation says which breaker ran | a library and a plugin |
| the **metric** | a typeface is **few**, so it is a **node**, and the paragraph's typeface is an **edge**; a monospaced paragraph is a metric with an empty table | an edge and a kind |
| the **metric's runs** | a width belongs to a **run** of characters, not to a character: a ligature replaces its characters and a kerning pair adjusts the gap between them | two tables in a payload |
| the **payload** | the core bounds a command's parameters and says nothing about a **payload**, so every plugin that reads one has to write its own bounds | a library and two core bounds |
| the **file** | every bound was applied **after** the bytes were in memory, and the biggest input is a file: the bound that matters is on the bytes, checked before the read | a module and three caps |
| the **document** | the depth bound was the JSON **parser's** number and not the kit's, and a value built in memory is not bounded by the parser at all — and a stack overflow is not a panic, so the guard never sees it | three constants and four checks |
| the **stream** | every newline-delimited door read its lines without a limit, and the control channel had declared one for years without using it | a module and six call sites |

Eighteen ports, and one of them is not a port at all: it is the answer to
a question three of them asked.

---

## 53. The cheap undo: the states share what they have in common

§48 listed the cheap undo as the one item on that list that is a
**regression against the family's own trees**: they share their documents
and snapshot a pointer, and this kit cloned. The survey (§40) said the
same of all three: *"Copy-on-write is how all three make undo cheap."*

This section fixes that, and it fixes it the way the family does it.

### What changed

The document's nodes are now **shared**:

```text
Node.children:  Vec<Arc<Node>>
Document.nodes: Vec<Arc<Node>>
```

and a mutation goes through `Arc::make_mut`, so a node that is shared
with a state in the history is **copied on the way to being changed** —
and a node that is not shared is changed in place. Copying a document
is now a matter of copying a vector of pointers.

The rule that makes it safe is the one `make_mut` gives for free: **a
state keeps what it was**, because the state holds a reference and the
mutation copies the node rather than changing it. That is not a
convention; it is what the type does.

### The measurement

The counting allocator (§32) is installed in the test binary, and the
numbers are the difference between two readings. Twenty edits, on a
document of two sizes, **before and after**:

| the document | before | after |
|---|---|---|
| 10 nodes | 2344 allocations | **764** |
| 400 nodes | 33560 allocations | **780** |

The same number of edits, on a document **forty times** the size, cost
**2.2 %** more allocations after the change and **fourteen times** more
before it. At four hundred nodes the change is **forty-three times**
fewer allocations for the same work.

That is the claim the gap list made and could not back: the cost of an
edit **does not depend on the size of the document**.

Two tests hold it: one that says a state and the document **share** the
node they have in common — a fact about the pointers, `Arc::ptr_eq`,
and not about the time — and one that says the cost of twenty edits on
a document forty times the size does not go up.

### What it did not fix

Honesty about the rest of it:

- **The comparison is still linear.** The session compares the document
  with the snapshot it took before the command, so that a command that
  says `mutates: false` and changes the document is caught. That
  comparison walks the document. The *memory* is shared now; the *time*
  to answer "did anything change?" is not.
- **The top level is copied.** A state copies the vector of top-level
  nodes — the pointers, not the nodes. A document with ten thousand
  top-level nodes still copies ten thousand pointers per state. The
  family's trees have a handful of spreads at the top and share those;
  this kit has one vector and shares the parts under it.
- **The file format did not change.** A node is written the same way it
  was, and the `Arc` is the document's business and not the file's.

### What it cost

- the node storage: two fields and the functions that reach them;
- one method for callers: `Node::add_child`, so that a plugin that
  moves a node under another one **does not know about the sharing** —
  the SDK's own group command uses it, and so does a test;
- **nothing else.** No command, no kind, no format version, and no
  plugin changed: the batteries that add nodes, mutate payloads and read
  trees are the same code they were.

### And what it is

The last of the family's three answers that this kit had not taken. The
family's trees make undo cheap by sharing; this kit now does the same,
and the measurement is in the tree rather than in a comment.

---

## 54. The gate: the rules of this tree, checked by a program

§48 said the one thing that is not a code gap: *"The mutation suite is
CI-only by the machine's rule, so no claim in this README has been
measured by mutation."* Underneath that sentence there was a bigger
problem, and this section is about it.

### The problem: `v2/` has rules and no gate

The outer tree has gates of its own — `cargo xtask lists` checks the deny
attribute in every crate root, and `cargo xtask layers` checks the
layering — and **none of them reach into `v2/`**, because `v2/` is
excluded from the outer workspace. So the rules this tree lives by were
**rules and not gates**: they were true, and nothing said so. A rule
that nothing checks is a habit, and a habit is gone the day somebody is
in a hurry.

So the tree needed its own gate, and it is `crates/craftkit-gates`.

### The three rules

1. **The deny attribute is the house one.** Every crate root carries the
   same six lints, in the same order. The comparison ignores the
   formatting — one line and six lines are the same six lints — but not
   the order: a deny list that varies between crates is a deny list
   nobody can reason about.
2. **The layering holds.** A crate may depend only on crates in
   **strictly lower** layers, and the table of layers is in the gate
   itself: a crate that is not in it is a finding, so a new crate has to
   be given a layer rather than quietly floating. The apps are exempt —
   they depend on everything, by design, and every real layering
   mistake has been made in a library.
3. **A plugin is a plugin.** A plugin may not depend on an application,
   and may not depend on another plugin. The seam is the SDK, the macro
   and the libraries, and nothing else.

### What it found the day it was written

The gate found three real things on its first run, and one of them was
about the gate itself:

- **The two binary-only applications had no deny attribute at all.**
  `craftkit-hello-server` and `craftkit-mcp-server` are packages with no
  library, so their root is `src/main.rs`, and nothing had put the
  attribute there. Production code, unprotected.
- **The gates crate was not in the layering table** — because it had
  just been written and the table is written by hand, which is exactly
  what the rule is for.
- **A dev dependency is a testing arrangement, and the rule had to say
  so.** The door's loader tests need a shared library to open, and that
  library is a plugin; the plugin's own tests need the shared library to
  be built first. Neither ships anything, so a **dev** dependency is
  exempt from the layering and from the plugin rule — and the gate now
  says that in its own words rather than leaving it to be guessed.

### The measurement

A gate that is only ever run on a good tree has never been shown to
**bite**. So the gate is a **function** — `check(root) -> Vec<Finding>` —
and the tests point it at trees that are broken **on purpose**:

| the broken tree | what the gate said |
|---|---|
| a deny attribute with fewer lints | the house lints, and where |
| a crate with no deny attribute | no deny attribute |
| a crate that depends on a **higher** layer | which crate, and which layer |
| a crate that depends on the **same** layer | the same |
| a plugin that depends on **another plugin** | the plugin |
| a plugin that depends on an **application** | the application |
| a crate that is **not in the table** | not in the layering table |
| a plugin that depends on the SDK and a library | **nothing** — that is allowed |

Eight trees broken on purpose and one that is right, and the gate says
the right thing about every one of them. That is the mutation
instrument, and it needed neither a copy of the tree nor a change to
it: the broken trees are built in a temporary directory, in the test.

### What it did not settle

- **The code mutation is still CI-only.** The outer tree's mutation job
  breaks the **code** and checks that the tests catch it; that is a
  seventy-five-minute run on the runner and the machine's rule keeps it
  there. What §54 measures is the **gates**: that each rule has teeth.
  The two are different instruments and only one of them has been run
  here.
- **The table is written by hand.** That is on purpose — the outer
  tree's table is the rule, and a table that is generated from the
  manifests cannot catch a manifest that is wrong. But it means a new
  crate is a two-step change: the crate, and the table.
- **The gate reads text.** It parses the manifests and the crate roots
  rather than asking cargo, which is what makes it fast and hermetic —
  and what makes it a *reading* of the tree rather than the tree
  itself. A manifest written in a form the reader does not know would
  be read as no dependencies at all.

---

## 55. The other door: a session you can drive from another program

§48 listed the **control channel** as not built, and said why: *"The MCP
server is the other door, and it is built."* That was true of the
*agent's* door. The family's applications have a second one — a channel
over a socket, for a program that is already running — and this tree did
not have it.

So it does now: `crates/craftkit-control`.

### What it is

A **session** behind a **door**: the server holds the session and
answers, and another program asks. The wire format is the family's —
newline-delimited JSON, one object per line:

```json
{"id": 1, "token": "…", "method": "engine.execute", "params": {"command": "node.add"}}
```

and the reply says whether it worked, in a machine-readable `kind` when
it did not:

```json
{"id": 1, "ok": false, "message": "there is no method `engine.explode`; …", "kind": "unknown-method"}
```

| the method | what it does |
|---|---|
| `ping` | whether the peer is alive |
| `engine.describe` | the command catalogue and the kinds |
| `engine.execute` | run a command |
| `session.state` | what the session is holding |
| `diagnostics.list` | the log and the journal |
| `shutdown` | stop the server |

### The four things the port found

- **Two doors, one session, and the same answers.** The MCP door is
  stdio and the control door is a socket, and a client that uses either
  gets the same registry, the same commands and the same outcomes. The
  sharpest evidence is the **waiting command** (§50): a command that
  asks while it runs is answered over the control channel in exactly the
  way the MCP door answers it — the answer comes back in the
  parameters, under the question's name, and the command goes on. The
  seam is in the session, so every door inherits it.
- **The catalogue over the wire is the command list.** §41 said the MCP
  surface *is* the registry; that holds for the second door too, and the
  catalogue is the same `CommandSpec` list the buttons are made of. There
  is no second description of the commands anywhere.
- **The `ui.*` methods are not here, and their absence is the finding.**
  The family's channel has `ui`, `ui.click`, `ui.screenshot` and
  `ui.render`, because the family has windows. This tree has no window
  (§16), so those methods would answer "there is no method" — and a
  method that is not there says so **by name**, listing the ones that
  are, rather than doing nothing quietly. That is the family rule about
  a closed set of methods, kept.
- **The token is on the first request of the connection, and the
  comparison does not stop early.** The family puts the launch token on
  the first request and nowhere else; a connection without it is closed.
  The comparison here walks the whole string, so the time it takes does
  not say how much of the token was right. A line is at most a megabyte:
  a request that is a gigabyte long is not a request, and the reader
  refuses it rather than reading it.

### What it cost

- one crate, **twenty lines of it** being protocol and the rest being
  the dispatch;
- **nothing in the core**: no command, no kind, no session method, no
  format version. The channel reaches the session the way the MCP
  server does — through `session.execute` — and through nothing else.

### What it does not settle

- **There is no application that serves it yet.** The crate is a door
  and a client; the program that opens the door and holds the window is
  the app's business, and this tree has no app with a window.
- **The crash reports are not a separate list.** The family keeps a list
  of crash reports; here a panic in a command comes back as the error of
  that command, and the log (`diagnostics.list`) carries it. The guard
  is the same one (§38); only the *shape* of the report is different.
- **The announce file is a convenience, not a discovery protocol.** The
  server can write its address and token to a file, so that another
  program can find the door. Where that file lives, and who is allowed
  to read it, is the host's business and not the channel's.

---

## 56. The typed parameter, and a claim that was wrong

§52 said, in its own words, that the typed value **cannot** be a command
parameter: *"the kinds a parameter may take are the core's — text,
number, bool, a node reference — and a plugin cannot add one."*

**That claim was wrong**, and this section is the correction, because a
README that keeps a claim it has stopped believing is worse than one
that never made it.

### What was actually true

The seam was open the whole time. The trait is the SDK's — it is called
`ParamKind` — and it is **not sealed**: nothing stops a crate from
implementing it for its own type. The derive has been generic over it
since the beginning; it writes `<#ty as ParamKind>::read(params, name)`
and never mentions what `#ty` is. So a library could implement
`ParamKind` for its own `Length` from the day the trait was written,
and a command could take a `Length` and not a number.

What nobody had done is *use* it, and what nobody could do was **name
the kind in the schema** — because the schema's kinds are the core's
list, and a length is a number, and `number` says nothing about what
the number means.

### The constrained change

Two optional constants, two optional fields, and nothing else:

- `ParamKind` gains `NAME` and `UNIT`, both `Option<&'static str>`,
  both defaulting to `None`. **Every existing kind is untouched**: a
  kind that says nothing says nothing, and the schema is exactly what
  it was.
- `ParamDoc` carries them, and two builders put them on.
- The derive passes them through, for a required parameter, an optional
  one and an either.
- The **ecosystem's schema** shows `type_name` and `unit`; the **MCP
  door's** schema shows `title` (which is what a JSON Schema calls a
  thing's name) and `unit`.
- The **door** carries them too — the parameters travel as JSON, so a
  plugin that is not part of the program can still say that its
  parameter is a length. No change to the ABI's shape.

### What it bought

An agent is told what the number is:

```json
{"type": "number", "title": "length", "unit": "mm", "description": "How long it is: …"}
```

`25` on its own says nothing — twenty-five of what? The schema now says
**millimetres**, and the command's parameter is the library's `Length`,
so the range check happens in the *reading*: a length of a billion
millimetres is refused before the command's body runs, and the body
cannot mistake a length for a factor.

And a person may type `2.5cm`: the reader takes a number in the unit
the schema says, **or** a string with a unit of its own. The schema
does not promise the string form — the *shape* is a number, and that is
what it says — but the help text does, and the reader accepts it.

### The library moved

`craftkit-numeric` was at L0 with no dependencies at all, and it is now
at **L3**, next to the vector geometry library, because implementing
`ParamKind` means speaking the SDK's types. That is the same trade the
vector library made, and the layering table says so.

### What it did not settle

- **The shape is still the core's.** A length is a number in the
  schema, and a plugin cannot invent a JSON Schema type: the *shape* is
  `number` and the *name* is `length`. A caller that validates strictly
  against the schema sends a number, which is right, and a caller that
  reads the title and the unit knows what it means.
- **A plugin over the door names its kinds only in the parameters.** The
  catalogue's *commands* still carry the core's shape, and a kind that
  is a plugin's own has nowhere to be named in the command's own record.

### And the correction, in the place it was made

§48's list is re-checked below, and the typed parameter comes off it.
What is left on that list is the **version a file is written at**, which
is a decision about the format's contract and not a seam.

---

## 57. The version a file is written at, and the bug the round trip found

§51 found the rule in the family's page layout application: it writes
the **oldest** version that still represents the document, so that old
readers keep working. §48 listed it as the one thing that was honestly
not there.

This section builds it — and the round trip that decides it found a bug
in the format that had been there since the relation's payload arrived.

### The rule

A file is written at the **oldest version that loses nothing**.

How the version is decided is the interesting part: **nothing is
declared and nothing is configured**. The document is written at each
version of the family, from the oldest to the newest, and read back.
The first version whose round trip gives the same document back is the
one. The versions are *tried*, and the one that keeps everything wins.

So the version is **derived from the document**, and here is what the
derivation says:

| the document has | it is written at |
|---|---|
| a plain node | **1** — the first version can say this |
| a muted node | **2** — the second version is the one with `muted` |
| a child | **3** — the third version is the one with children |
| a payload of its own | **4** |
| a relation | **5** |
| a stream | **7** |
| a relation that says something | **8** — the payload of a relation is the eighth version's field |
| a hidden node | **9** — the flag is the ninth version's field |
| a locked node | **10** — the flag is the tenth version's field |

A plain document is written at the **first** version. A document with a
stream is written at the seventh. And the bundle says which version it
wrote, in the document's own tag ✓.

### The bug

The round trip found it on the first run, and it was a real one: **a
relation's payload was lost from every file**.

The commit that gave the relation its payload (§339) said the right
thing — *"the older writers take the field off again, because a file of
the seventh version must not carry a field the seventh version does not
know"* — and then wrote it the other way round:

- the **seventh** version wrote the relations *with* their payloads, a
  field the seventh version does not know;
- the **eighth** version — the one that *has* the payload — took the
  payload off again.

So the payload never reached a file: the version that could write it
dropped it, and the version that dropped it wrote one. A relation that
said something said it only until the document was saved.

The fix is two lines in the two writers, and two tests hold it: a
relation's payload survives the file, and a file of the seventh version
does not carry it.

This is the **first bug** this tree's own features have found, and it
was found by a feature that was built to answer a different question.
That is worth saying plainly: the round trip was not written to find a
bug, and it found one.

### What it cost

- one function in the format module — `write_document_oldest` — and the
  bundle's writer uses it;
- the bundle's `write` takes the migrations now, because the round trip
  needs them;
- **nothing in the format's shape**: the eight versions, the eight
  writers and the seven steps are exactly as they were.

### What it does not settle

- **The bundle's own version is separate.** A bundle carries its own
  version number, and that is still the newest one: the *document*
  inside it is written at the oldest version that keeps it, and the
  bundle around it is the newest bundle there is. Those are two
  versions, and only one of them is about the document.
- **The version is the oldest that *this* program can read back.** A
  program that knows fewer versions writes a **newer** file, because it
  cannot read back what it does not know — which is right: a file must
  be readable by the program that wrote it.
- **The bytes a node's stream points at are not in the comparison.**
  They travel beside the document in the bundle (§25), and no version of
  the document file can lose them.
- **The cost is a read and a write per version.** Eight versions, so
  sixteen operations on a save. That is the price of a rule that is
  decided by trying rather than by declaring, and it is cheap enough to
  be worth it.

---

## 58. The twelfth battery: a paragraph, and the font the kit does not have

§40 surveyed **craft-libs** and found, among twenty-six crates, a
`craft-text`: hyphenation in forty-eight languages, UAX #14 line breaks,
UAX #9 bidi, and a **line breaker** on Knuth and Plass's model — the
optimal one and the greedy one, on boxes, glue and penalties, with the
elasticity in three tiers.

That breaker is the part this kit had never had. Every port so far has
brought a **document model**; this one brings an **algorithm**, and an
algorithm is a different kind of thing to port: it is not a kind of node
and not an edge, it is **arithmetic**, and arithmetic belongs in a
**library**.

So this arc is two crates:

- `crates/craftkit-text-layout` — the **library**: the model, the
  optimal breaker and the greedy one, and the refusal of a hostile
  width or measure;
- `crates/craftkit-plugin-text` — the **battery**: a paragraph, its
  words, the measure it sits in, and the lines it breaks into.

### What was ported

The model is three kinds of item and one rule each:

| the item | it is | and |
|---|---|---|
| a **box** | fixed material — a word, a glyph | it is never a break |
| **glue** | elastic space | it is a break when it follows a box, and the space at the break is dropped |
| a **penalty** | a break opportunity with a cost | a hyphenation point is a penalty that adds a hyphen's width when it is taken |

The **optimal** breaker minimises the whole paragraph at once: it is a
dynamic program over the break opportunities, and each break keeps the
best path to it for each fitness class of the line that ended there —
tight, decent, loose, very loose — and, when there is a limit on
hyphenation, for each number of flagged breaks in a row. The **greedy**
breaker takes as much as fits on each line and never fails. The library
says **which one ran**: `Breaks::method` is `Optimal` or `Greedy`, and
that answer is in the plan.

### The seven things the port found

- **The kit has no font, so the widths come from the caller.** A line
  breaker needs the advance of every glyph, and an advance comes from a
  typeface — and this tree has no typeface and no asset of any kind,
  because the outer tree's rule is that everything on screen is drawn in
  code. So the paragraph carries an **advance** (the width of one
  character) and a **space**, in millimetres, and the caller says what
  they are. The moment a font arrives it supplies those two numbers and
  nothing else changes: the breaker, the plugin, the document model and
  the file format are all untouched by it. — **Answered in §128**: the
  kit has its own typeface now, drawn in code, and the widths **still**
  come from the caller — because the widths that matter to a paragraph
  are the ones in its metric.
  This is the first time in this kit that a **missing asset** shaped a
  document model, and the shape it took is honest: the metric is a
  **value in the payload**, not a hidden assumption.
- **The words are the payload, and the string is not.** The word
  processor stores a paragraph's text once, as a string, with runs over
  it. Here the string would be a second copy of what the words already
  say, so the **words are stored** — a list in the payload — and the text
  is split on whitespace when the paragraph is made and never put back
  together. That is §49's rule ("what is numerous is in the payload")
  applied to a thing that is numerous *and* a list.
- **The measure is the paragraph's place, not the paragraph.** A
  paragraph's measure is the width of the column it sits in, and the
  column is a **place**. So the measure is set by a **command**
  (`text.paragraph.measure`), and a paragraph with no measure yet is a
  paragraph that has not been placed: it is in the document and it has
  **no lines at all**, and the evaluation says so with a **null** rather
  than a zero — a paragraph with no lines and a paragraph of no lines are
  not the same thing.
- **The lines are derived, and the derivation says which way it went.**
  The plugin does not choose the breaks: it turns the words into boxes
  and glue — a box of `characters × advance` for each word, a glue of
  `space` between them — and hands them to the breaker. The plan carries
  the lines, their widths, their ratios, and **the method**, so a caller
  can tell a paragraph that was broken optimally from one that fell back
  to the greedy breaker. A derived value that says how it was derived is
  worth more than one that does not.
- **The optimal breaker needs the whole paragraph at once, so the
  paragraph is bounded.** It is quadratic in the number of break
  opportunities, and a paragraph with ten thousand of them would take a
  while. So the library names the limit — `MAX_BREAKS`, five thousand —
  and past it the greedy breaker runs and says so. This is a **new kind
  of limit in this kit**: not a limit on a number a caller gives, but a
  limit on the **size of a document**, and the answer is that the document
  is still written and read; only the *choice* of breaks is made the
  cheap way.
- **A varying measure is not built, and now it is named.** One
  paragraph has one measure — the width of its column. A first line that
  is narrower (an indent, a drop cap) makes the optimal breaker quadratic
  in the **number of lines** as well as in the number of breaks, because
  the measure of a line depends on how many lines came before it. The
  library says so in its own documentation rather than pretending, and
  the greedy breaker — which does not care — can already do it.
- **Hyphenation was a dictionary's business, and now the kit does one
  language of it.** The breaker knows a **hyphenation point** — a
  flagged penalty — and it enforces the limit on how many flagged
  breaks may follow each other, as a **hard constraint**: a break that
  would exceed it is not taken, in both breakers. *Finding* the points
  was the dictionary's work, and the family's dictionary carries
  forty-eight languages — one of them under a licence that is not this
  tree's. So the dictionary is not here, and the kit does the next
  thing: it carries the **patterns** for English and asks them where a
  word breaks (§145).

### What it cost

- one **library** (`craftkit-text-layout`) with no dependencies at all —
  the first crate in this tree that speaks neither the SDK nor anybody
  else, and the first **library** that is not an arithmetic over the
  kit's own types;
- one **battery** (`craftkit-plugin-text`) with two commands, one kind,
  one evaluator and one panel;
- **nothing in the core.** No new parameter kind, no new relation, no
  new field in the format: a paragraph is a node with a payload, exactly
  like the prose battery's block (§49).

### What it does not settle

- **The glyphs.** The advance is one number for every character, so a
  paragraph set with it is a **monospaced** paragraph. A font would give
  every glyph its own advance, and the model would not change — but the
  model would have to carry **per-glyph** widths, and the plugin's
  payload would grow from two numbers to a list. That is the port's
  **next** step, and it is a port, not a change.
- **The justification.** The library rates a line and says how much it
  has to stretch; it does not **place** the words. Placing them needs
  the same advances, so it is the same missing thing.
- **The bidi and the line-break opportunities.** UAX #9 and UAX #14 are
  in the family's `craft-text` and are not here: they find *where* a line
  may break, and this port only finds *how* the lines are best once the
  opportunities are known.

---

## 59. The metric is a node, and the typeface is an edge

§58 built the line breaker and put the **metric** — the width of a
character and the width of a space — in the **paragraph's payload**. It
worked, and it was wrong: a metric is not a property of a paragraph.

This section moves it, and says what the move found.

### The move

A metric is **few** — a typeface is shared by every paragraph in a
document, and it has a name — so a metric is a **node**, and a paragraph
says which metric it is set in with an **edge**:

| | before (§58) | now |
|---|---|---|
| the widths | in the paragraph's payload | in a **metric node's** payload |
| the paragraph's typeface | — | an **edge**, `text.set_in` |
| a monospaced paragraph | a paragraph with one advance | a metric with an **empty table** |

The commands follow: `text.metric.add` puts a metric in the document,
`text.paragraph.add` takes the metric the paragraph is set in, and
`text.paragraph.metric` sets it again — which **replaces** the edge,
because a paragraph is set in one metric and not in two.

### What the move found

- **A thing with an identity is a node, and a thing without one is a
  payload.** The first version of the battery could not tell the two
  apart, because a metric *looked* like a number the paragraph carried:
  one number for the advance, one for the space. The moment the metric
  became a **table** — a width for every character that has one — it
  became a thing with a name, a shape and a life of its own, and the
  rule fired: **few is a node**. The PDF's dictionaries, the CAD's
  layers, the prose's styles and the layout's stories all said the same
  thing, and this is the **fifth** kind of edge in this kit.
- **The dependency moved with it, and the plan still gets it right.**
  Before, the paragraph's lines depended on the paragraph's own
  payload. Now they depend on **another node**, across an edge — and the
  evaluation follows the edge, so changing the metric changes every
  paragraph's lines **and touches none of the paragraphs**. There is a
  test for exactly that: the metric is narrowed, the same four words go
  from two lines to one, and the paragraph's payload is byte for byte
  what it was. This is the "derived values are derived" rule (§43) with
  the dependency **outside** the node, which is the first time in this
  tree that has happened.
- **A monospaced paragraph is not a special case; it is a metric with
  an empty table.** The general metric has a width for every character
  that has one and a **default** for the rest; the monospaced one has
  only the default. One code path, no branch, and the degenerate metric
  is the one everybody starts with. The finding is that the *general*
  case is no harder than the special one — the table is a payload, and a
  payload is a value.
- **The kit has no font, and that is not the same as having no
  metric.** What a line breaker needs is a **width**, and a width is a
  number. A font is glyphs — outlines, kerning, ligatures, hints — and
  this tree may not have one. The two are separable, and the separation
  is what makes the port legal: the metric is **numbers**, and numbers
  are data and not artwork, so the tree's no-asset rule and the missing
  font do not fight each other. The moment a font arrives it supplies
  the table, and nothing in the model moves. — **Answered in §128**:
  the typeface arrived, and nothing in the model moved.
- **A table's key is one character, and the reader keeps what it does
  not understand.** The command refuses a key that is not a single
  character and a width that is not a width; the **reader** — which may
  be reading a file somebody else wrote — leaves such a key out, and
  the character falls back to the default. That is the reader's rule
  (§43) applied to a table instead of a document.
- **The paragraph's payload now holds two things of two kinds, and
  that is as far as it goes.** The **words** are numerous and in the
  payload; the **measure** is the paragraph's *place* and in the
  payload; the **typeface** is few and is a node; and the **link** is an
  edge. Four kinds of thing, four homes, and the payload holds only the
  two that belong to the paragraph alone.

### What it cost

- **nothing in the core** — again. A metric is a node like any other,
  and the edge is a relation like the PDF's or the prose's;
- **two** commands and **one** kind added to the battery, and a change
  to two commands that already existed;
- the plugin's tests grew from eight to sixteen, and the studio's from
  nine to twelve.

### What it does not settle

- **The glyphs are still not here.** A metric is a width per character,
  and a real typeface has a width per *glyph run*: a ligature is not
  the sum of its letters, and a kerning pair is not two widths side by
  side. The model would need the widths to come in **runs** rather than
  per character, and the metric's table would become a list of
  (characters, width) pairs. That is the port's next step, and it is a
  port.
- **There is no font *file* and no font *name* that means anything.**
  The metric is called what the caller calls it. — **Still true in
  §128**: the kit's typeface is a table of strokes in a source file and
  has no name to speak of.

---

## 60. A width is not a property of a character

§59 left a gap and named it: a metric was a width **per character**, and
a real typeface is not that. The letter pair "fi" in a typeface with the
fi ligature is **one glyph**, and the pair "AV" in a typeface with
kerning is **two glyphs that are closer together**. Neither is a sum of
character widths.

This section gives the metric what it needs, and says what the port
found.

### The three tables

A metric is now **three tables**, and the three are applied in three
different places:

| the table | it is | what it does |
|---|---|---|
| the **advances** | a character → millimetres | the width of a character that has one |
| the **runs** | two or more characters → millimetres | **replaces** its characters: a run is one glyph |
| the **pairs** | exactly two characters → millimetres | **adjusts** the gap between them: a pair is two glyphs |

A word is read from the left: at every place the **longest run** that
matches is taken and replaces its characters, and at every **boundary**
the pair's adjustment is added. The adjustments may be **negative** —
that is what a kerning pair is for — and the widths may not.

Two rules have teeth, and both are in the command rather than in the
reader:

- a **run** is two characters or more. A run of one character is
  refused, because that is an advance, and the metric has a table for
  those;
- a **pair** is exactly two characters. A pair of three is refused,
  because that is a run.

### What the port found

- **A width belongs to a run of characters, not to a character.** This
  is the second time in this tree that a number turned out to be about
  something else than it looked (§38's panic, §52's unit). The metric's
  table was not a table of widths; it was a table of widths *for single
  characters*, which is a special case of the real thing, and the real
  thing needed **three** tables and a rule about which one wins.
- **The longest run wins, and the rule is the model's.** Shaping turns a
  string into **clusters**, and the cluster is the longest thing that
  matches. So the reader looks for the longest run first, at every
  place, and takes it. There is no separate "shaping" step: the width
  of a word *is* the reading of the word, and the reading has a rule.
- **A run and a pair are different things, and the model says so.** A
  ligature is **one glyph**: it replaces its characters, and it has a
  width. A kerning pair is **two glyphs**: it replaces nothing, and it
  has an *adjustment*. One table with widths and one table with
  adjustments is not tidiness — it is the difference between the two
  things, and a model that had one table for both would have to encode
  "this replaces" and "this adjusts" in the numbers.
- **A kerning pair may be negative and a width may not.** The rule is
  in the command and it is a *sign* rule, not a rule about the size of
  the number: an adjustment is a difference and a difference may go
  either way. A width below nothing is refused, because a width below
  nothing is not a width.
- **The reader leaves out what it does not understand, and the command
  refuses it.** A metric that arrived in a file may hold a run of one
  character and a pair of three: the reader drops both, and the letters
  fall back to the default. A metric that is being made cannot: the
  command says why, and says which of the two rules was broken. This is
  §43's reader rule and §59's command rule, in the same table, with
  the same difference between them.
- **The bound is on the run, and it is a bound on the work.** A run is
  looked for at every place of a word, and the longest one wins, so the
  number of characters a run may have is the number of places that are
  looked at. The metric's runs are at most **eight** characters, and the
  constant is named: `MAX_RUN`. A typeface with a longer cluster than
  that is a typeface this model does not shape — and the model says so
  rather than searching forever.

### What it cost

- **nothing in the core**, and nothing in the library: the breaker takes
  item widths, and the widths are still widths;
- **two tables** in the metric's payload, two optional parameters in
  the command, and the word-width function, which is no longer a sum;
- seven tests in the plugin and two in the studio.

### What it does not settle

- **A cluster is not a glyph.** Real shaping also decides *which*
  glyphs a cluster is made of, and where each one is put: a mark's
  position, a contextual form, a script's own rules. This port has the
  **width** of a cluster and not its **shape**, so a paragraph set with
  it is measured correctly and would not be *drawn* correctly — and this
  tree draws nothing, so that is not a gap here yet.
- **The adjustment is one number per pair.** Real kerning is per pair
  **and** per size, and a typeface's tables are large. The model's
  table is a payload, so it can be as large as the document can carry
  — but nothing bounds it, and nothing says how many pairs a metric
  may hold.

---

## 61. The payload is the untrusted hole

§59 and §60 gave the metric three tables, and the tables are read out
of a node's **payload**. Nothing about that is safe by itself: the
payload came from a file, and the kit's rule is that input-derived
numbers are hostile — *check finiteness, bound sizes, cap loop counts*.

The core enforces that rule for a command's **parameters**. It does not
enforce it for a **payload**, because the core has no opinion about
what a payload holds: that is the plugin's business, and the plugin
reads the payload itself.

Which means: **every plugin that reads a payload has to write its own
bounds**, and a plugin that forgets is a plugin that will read a table
of a million entries into a map because a file said so.

This section closes that hole the way the kit has closed every other
one: with a **library**.

### The library

`crates/craftkit-payload` is a **bounded reader** for a JSON value. It
depends on serde_json and on nothing else — not even on the SDK — and
it reads:

| it reads | it is | and it refuses |
|---|---|---|
| `number` | a number | one that is not a number, or one beyond the limit |
| `text` | a string | one that is longer than the limit |
| `boolean` | a true or a false | anything else |
| `words` | a list of strings | a list that is too long, or a word that is too long |
| `table` | a map of keys to numbers | a table that is too big, a key that is too long, a value that is not a number |
| `table_lossy` | the same | the same, **except** that an entry it does not understand is skipped |

`Limits` is a value: the most entries, the longest key, the longest
string and the largest number. The **plugin** chooses them, and the
text battery's are in one place — a thousand entries, a sixty-four
character key, a four-thousand-character word, a number under a
million.

### What the port found

- **The payload is the hole the parameters are not.** The core bounds a
  command's parameters and says nothing about a node's payload, so the
  **reading** of a payload was the plugin's business and its **size**
  was the plugin's business too — which means it was nobody's until a
  plugin wrote it down. The answer is the same one the typed value
  gave (§39, §52): the core has no opinion about a payload's shape, so
  the mechanism is a **library** and not a change to the core.
- **A bound is not negotiable and a shape is.** The same table can be
  read two ways: `table` refuses an entry it does not understand, and
  `table_lossy` skips it. The **bounds bite in both**: a table with too
  many entries is refused either way, because a bound that can be
  traded away is not a bound. Which reading to use is the plugin's,
  because it depends on whether the reader must keep what it does not
  understand — and in this tree it must (§43).
- **Finiteness is the format's job, and the format has done it.** A
  JSON number is always finite: serde_json **refuses** a number that is
  out of range and writes a float that is not finite as `null`. The
  kit's rule asks for a finiteness check on every number from outside,
  and for a payload the format has already made it. What the reader
  checks is the **magnitude**, which the format does not know about.
  The library says so in its own documentation rather than carrying a
  check that cannot fire.
- **The core's own kinds are now bounded, and the bounds are safety
  nets.** A **text** parameter is refused above sixty-five thousand
  characters, and the **raw door** — a JSON value of any shape — is
  refused above a million values or sixty-four of nesting. Both are
  *looser* than the bounds a plugin sets on the values it takes: the
  numeric battery refuses a hundred thousand lengths itself (§52), and
  its refusal is the one a caller sees, because a count is a
  **meaning** and a meaning is the plugin's. The core's bound is for
  the value nobody thought about.
- **A bound on a number's magnitude is still the plugin's.** The core
  checks that a number is finite and nothing more, because the range
  of a number depends on what it *means*: a millimetre is bounded by
  the typed value, a sample count by the sound battery, and a
  coordinate by the CAD battery. The core cannot know, and guessing
  would be a unit in the core — which §39 said is the one thing the
  core must never have.
- **The paragraph's words are bounded too, and the refusal is
  visible.** A paragraph whose words are more than the limit allows is
  **not read**: it has no lines, and its evaluation says its word count
  is **null** rather than a number it cannot vouch for. A document that
  a file made enormous is not a paragraph anybody can lay out, and the
  kit says so instead of laying it out.

### What it cost

- **one library** (`craftkit-payload`), seventeen tests, and no
  dependency but serde_json;
- **two bounds in the core** — a text parameter's length and the raw
  door's size — and four tests;
- the text battery's reader went through the library, and the battery
  grew two tests of its own and one in the studio.

### What it does not settle

- **The payload is bounded by the plugin that reads it, and two
  plugins reading the same payload may bound it differently.** In
  practice one plugin owns a kind and reads its payload, so this has
  not come up; but the kit has no **shared** rule about how big a
  document's payload may be, and the file format does not check it
  either.
- **A document read from a file is not checked against these bounds at
  the door.** The reader refuses a payload it is asked to read, and the
  format's reader keeps what it does not understand (§43) — so a file
  with an enormous payload is read into memory and only then refused.
  The bound that would stop that is in the **format**, and the format
  has no opinion about payloads either.

---

## 62. The door that reads a file

§61 bounded the **values** in a document — a payload's tables, a
paragraph's words, a command's parameters. Every one of those bounds is
applied **after** the bytes are in memory, and for the biggest input
there is that is the wrong order.

A document arrives as a file, and a file arrives as **bytes**: a file of
ten gigabytes was read into a `String` before anything had a chance to
look at it, and the refusal that followed was a refusal of the
*reading* and not of the *file*. The memory had already been spent.

This section puts a bound where the bytes arrive.

### What was built

- **`craftkit_sdk::files`** — a **bounded read**: the file's own length
  is checked **before** the read, and a file that is too long is refused
  without being read. Two functions, one for bytes and one for text.
- **The bundle's counts are capped**: the number of files, the length of
  the document and the length of a path. A cap **refuses** the bundle;
  it does not read less of it. The two byte caps are the format's own
  and are named here so that a reader can look them up:
  `MAX_BUNDLE_DOCUMENT_BYTES` (sixty-four megabytes) and
  `MAX_BUNDLE_PATH_BYTES` (four kilobytes); the number of files is
  `MAX_FILES` (four thousand and ninety-six).
- **The doors use it**: `doc.open` (a document), `doc.open.bundle` (a
  bundle), `doc.save.bundle` (an asset), and `task.load` (a list of
  tasks). Each has a limit, and the limit is **the program's**: a
  document is sixteen megabytes, a bundle is two hundred and fifty-six,
  a task list is four.

### What the port found

- **The door was the hole, and every bound was in the wrong place.**
  §61's bounds all work on values that are *already in memory*, which
  is fine for a payload of a megabyte and useless for a file of ten
  gigabytes. The bound that matters most is on the **bytes**, and it
  has to be checked on the file's **length**, before the read. This is
  the kit's own rule — "bound sizes" — read in the order the input
  actually arrives in.
- **The bundle reader was safe by accident.** Its file count came out
  of the file, and the loop stopped because the **bytes ran out**, not
  because a rule said so. Running out of bytes is not a rule: it is the
  buffer saying no, and the count that caused the work was never
  capped. Now it is, and the cap **refuses** the bundle — because a cap
  that can be traded away is not a cap, and truncating a bundle into
  one that fits is a lie about what was read.
- **The limit is the program's, not the kit's.** A program that opens
  documents needs one limit; a program that opens bundles of media
  needs another; a program that opens video needs a third. The kit does
  not know which program it is in, so the limit is an **argument** and
  `files` is the mechanism everybody uses. This is the same answer as
  §61's payload reader: the mechanism is shared, the number is the
  caller's.
- **The format's depth was already bounded, by the format.** A document
  nested deeper than a hundred and twenty-eight levels is refused by
  serde_json's **parser**, so the tree the reader builds cannot be
  deeper than that and no separate bound is needed. It is the third
  time the format has done a check before the kit could (§61's
  finiteness, and now this): a JSON number is finite, and a JSON
  document is shallow. The kit's rule asks for both, and for a
  document the format has already made them.
- **A refused file is not a broken program.** The refusal says which
  file, how long it was and what the limit was — the same shape as every
  other refusal in the kit (§21): a value, not a panic, and not a
  message that says "error".

### What it cost

- **one module** in the SDK (`files`), five tests, and no new dependency;
- **three caps** in the bundle reader, two tests;
- **four doors** changed, and one test that writes seventeen megabytes
  and is refused.

### What it does not settle

- **A file that is not a regular file cannot be read through this
  module.** A pipe has no length to check, so it is refused — and a
  program that wants to read a pipe reads it another way and says so.
- **The bundle's own bytes are still the caller's business.** The
  bundle's files may be media, and the limit on the *bundle* is the
  program's; only the document inside it has a limit in the format
  itself.
- **Nothing bounds the tree's depth in the kit's own terms.** The
  format's parser bounds it at a hundred and twenty-eight, and that is
  the number in force — but it is the *parser's* number, not the kit's,
  and the kit has no way to say "this document is too deep for me".

---

## 63. The document's own bounds, and the number that was not ours

§62 ended with an unsettled line: *"nothing bounds the tree's depth in
the kit's own terms; the format's parser bounds it at a hundred and
twenty-eight, and that is the parser's number, not the kit's."*

This section makes it the kit's number, and finds that the parser's
number was the only thing standing between a document and a **crash**.

### What was built

The **document's shape** is now bounded, and the bounds are the kit's:

| the bound | it is | and it is |
|---|---|---|
| `MAX_DEPTH` | sixty-four levels | the deepest a document's tree may be |
| `MAX_NODES` | ten thousand | the most nodes a document may hold |
| `MAX_RELATIONS` | ten thousand | the most relations a document may hold |

They are checked in three places, and the three are the same numbers:

- **while the node is read** — the reader is recursive, and it counts
  the depth on the way down;
- **on the document that was read** — the node count and the relation
  count, checked on the tree the reader built;
- **on the document that is about to be written** — the bundle and the
  save command refuse a document that nobody could read back.

### What the port found

- **The depth bound was the parser's, and a parser's bound is not a
  rule.** serde_json refuses a document nested deeper than a hundred and
  twenty-eight levels — which is why nothing had ever gone wrong. But
  the parser's limit only applies to a value that was **parsed**. A
  value that was **built** — a command's raw parameter, a plugin's own
  JSON, a document assembled in memory — is not bounded by the parser
  at all, and the kit's reader is **recursive**: it would have walked
  as deep as the value went.
- **A stack overflow is not a panic, and the guard does not catch it.**
  This tree's panic guard (§38) turns a panic into an error, and the
  kit's rule is that no production code panics. But a recursion that
  runs out of stack does not panic: it is a **crash**, and the guard
  never sees it. So the depth is checked on the way down, and the
  refusal is a value like any other. This is the first time in this
  tree that a bound has been about the **stack** rather than about the
  memory.
- **The reader and the writer must agree, or the round trip means
  nothing.** §57 decides which version to write by writing the document
  and reading it back. If the reader refused documents the writer
  happily wrote, the round trip would fail for every document near the
  bound — and the version would be decided by the bound rather than by
  what the document holds. So both sides check the same numbers, and
  the numbers are the kit's.
- **The document's shape is the kit's business and the plugin's is
  not.** A plugin may add a kind of node, a payload and an edge (§12,
  §44) — and it may not make the document deeper or wider than the kit
  allows. That is the same seam as everywhere else: the plugin owns
  what a node **means**, and the kit owns how many nodes there may be.
- **A document over the bound is refused, not truncated.** A document
  with too many nodes is refused at the door and refused at the save;
  it is never trimmed to fit. A document that is quietly smaller than
  the file says is a lie about what was read, and §62's finding — that
  a cap which can be traded away is not a cap — says the same thing
  here.

### What it cost

- three constants and one method on the document (`check_bounds`), and
  the node reader's depth check;
- **four checks** in the format, the bundle and the save command;
- seven tests.

### What it does not settle

- **The depth of a document that is *built* rather than read.** The
  reader's bound applies to the reading; a program that assembles a
  tree in memory can build one deeper than the bound, and the bound is
  only checked when the document is read or written. The **writing**
  check catches it (§57's round trip reads it back) — but the program
  that built it had the stack in the meantime.
- **The bounds are the kit's, and the kit has no opinion about what a
  document is for.** Ten thousand nodes is right for a drawing and
  wrong for a spreadsheet with a million cells; the number is a
  **default**, not a fact, and a program that needs a different one has
  no way to say so.

---

## 64. The doors that read a line

§62 bounded the file that a document arrives in. The next door is the
same shape and it was worse: every **stream** in this kit speaks
newline-delimited JSON, and every one of those readers read a line
until its newline, with **no limit** on how much they would hold.

### What was built

- **`craftkit_sdk::lines`** — a **bounded line reader**: the line is read
  **byte by byte**, so the limit is checked *as the line is read* and
  not after the whole of it has been in memory. A line that is over the
  limit is refused, and the stream **ends**: the reader was cut off
  mid-line and cannot know where the next one starts.
- The **doors** use it: the plugin's wire in both directions (§34),
  the **control channel** (§55), the **MCP server** (§41) and the
  **bridge**.
- The limits are the **protocols'**: a megabyte for the plugin's wire
  and the bridge, four for the control channel and the MCP server (a
  tool's arguments may carry a whole document).

### What the port found

- **The protocol already had the number, and the reader did not use
  it.** The control channel has declared `MAX_LINE` — a megabyte —
  since it was written, and its reader read until a newline. A limit
  that is declared and not enforced is **worse than no limit**: it says
  the protocol is bounded when it is not, and a reader of the code
  believes it. Now the number is the reader's bound ✓.
- **A reader that checks the length afterwards has already spent the
  memory.** The bound that matters is the one checked *while* the
  bytes arrive, so the mechanism reads a byte at a time and stops at
  the limit. A line of a gigabyte costs a megabyte of memory and then
  a refusal — which is the difference between a program that survives
  a hostile peer and one that does not.
- **A refused line ends the stream, and that is the rule.** The reader
  was cut off in the middle of a line and cannot know where the next
  one begins, so the door closes. This is §62's rule again — a refusal
  that can be resynchronised is not a refusal — and it is the same
  shape as the file: the bytes that were read are gone.
- **A stream's peer is alive, and a file's is not.** A file arrives and
  ends: its size is known before it is read, which is what §62's bound
  exploits. A peer can send **forever**, and the only thing that stops
  it is a number the reader counts. That is the whole difference
  between the two arcs, and it took two arcs to find it.
- **The limit is the protocol's, and the mechanism is the kit's.** Five
  doors, five numbers, one reader ✓ — the same answer as §61's payload
  reader and §62's file reader: the mechanism is shared and the number
  belongs to the caller.

### What it cost

- one module in the SDK (`lines`), six tests, and no new dependency;
- **six call sites** changed — the plugin's wire in two directions, the
  control channel in two, the MCP server and the bridge;
- one test in the control channel: a peer sends a line longer than the
  protocol allows, and the server refuses it rather than reading it.

### What it does not settle

- **A line is not the only thing a stream carries.** The MCP protocol
  frames its messages with headers and a body, and a body that is a
  megabyte of JSON is a line of a megabyte here — but a protocol that
  frames differently would need its own bound, and the mechanism reads
  **lines**.
- **The refusal says how long the limit is and not how long the line
  was.** The reader stops at the limit, so it does not know how much
  more was coming, and a message that says "the line is longer than a
  megabyte" is true and is all the reader knows.

---

## 65. The session's own memory, and the counts that were not bounds

§63 bounded the **document**. This section bounds the things a session
holds *beside* the document — and it found that every one of them had a
**count** and no **number of bytes**, which is the difference between a
limit and a hope.

### What was built

- **The document has a byte budget.** `MAX_DOCUMENT_BYTES` is
  thirty-two mebibytes, and it is the **same name and the same number**
  the family uses: a document is a document in either tree, and a bound
  that is written down twice with two numbers is two bounds.
- **The estimate never under-estimates.** A budget a document can lie
  about is not a budget, so the estimate counts the **capacities** and
  not the lengths, and it counts the children, the relations and the
  assets too. It is walked with a stack and not by recursing: a bound
  that is checked by overflowing the stack is not a bound (§63).
- **The history has a byte budget beside its count.**
  `MAX_HISTORY_BYTES` is the document's budget times
  `HISTORY_EXPANSION` (two) — sixty-four mebibytes, and the same name
  and the same number the family uses. Each state's cost is measured
  **when the state is made**, so the pruning is a **subtraction** and
  not a walk of the tree.
- **The journal has a byte budget beside its count.**
  `MAX_JOURNAL_BYTES` is eight mebibytes. It is the **kit's own** name
  and the kit's own number, and it is named here: a bound a reader
  cannot look up is not a bound.
- **The tasks have a byte budget beside their count.**
  `MAX_TASKS_BYTES` is eight mebibytes, and a task that is still
  **running** is never dropped for it.
- **The log is bounded.** `MAX_LOG_ENTRIES` is two hundred and fifty-six:
  the log is written on every command, and a session that runs for days
  would otherwise hold every line it ever wrote.
- **The bounds a program owns are the program's.** The journal's
  budget, the task registry's budget and the history's budget can be
  said out loud (`set_journal_budget`, `TaskRegistry::with_budget`,
  `History::with_budget`). The document's byte budget is **not** the
  program's: a document is a document in either tree.

### What the port found

- **A count is not a bound.** The history held two hundred and fifty-six
  states, the journal a thousand entries and the registry a thousand
  tasks — and every one of those held something whose size the caller
  chose. Two hundred and fifty-six states of a thirty-megabyte document
  are **seven and a half gigabytes**, and a thousand journal entries of
  a megabyte each are a gigabyte. The counts said "a lot" and the
  memory said "everything".
- **The bound that binds is the byte one, and it binds long before the
  count does.** A document over half a megabyte keeps **two** states in
  the family, not a hundred and twenty-eight — and the number that
  decided it was never the count.
- **The budget is derived, not written down.** The history's budget is
  the document's budget times an expansion, so that raising the
  document's budget moves the history's with it. Two caps with the same
  value and no relation between them drift apart the moment one of them
  is changed — which is the reason the family's two caps are derived
  from the model's.
- **The measure is made when the thing is made.** A pruning that has to
  measure a document to know what to drop is a pruning that walks the
  whole tree on every push; a pruning that subtracts what the entry cost
  is a subtraction. The family measures the entry's bytes at push time
  for the same reason, and the two trees now say the same thing.
- **A budget may cost the history, and never the document.** The state
  that is current is not dropped for the budget: it holds the document
  the session is working on, and a bound that drops the working
  document is not a bound, it is a data loss.
- **Work that has not been accounted for is not dropped.** A task that
  is still running is not dropped for the byte budget: it is work whose
  result nobody has collected yet, and losing it loses the handle the
  other end knows it by.
- **Two trees, one number, and the names that are shared are the
  family's.** `MAX_DOCUMENT_BYTES` and `MAX_HISTORY_BYTES` are the
  family's names with the family's numbers, so that a reader of either
  tree can look them up. The caps that are the kit's own — the
  journal's, the tasks' — are **private** and are named here, because a
  name that means two things in two trees is a name that means nothing
  in either.

### What it cost

- two constants and two methods on the document (`estimated_bytes` and
  the byte check inside `check_bounds`), two constants and the
  accounting on the history, one constant and one method on the
  journal, one constant and one method on the task registry, one
  constant and one method on the session;
- one new test file with **ten tests** (`tests/bounds.rs`), beside the
  seven that hold §63's bounds.

### What it does not settle

- **A document built in memory is still bounded only when it is
  written.** This is §63's unsettled line, in the same shape: the byte
  budget is checked when the document is read and when it is written,
  and a program that assembles a thirty-three-megabyte document in
  memory holds it until it tries to save it.
- **The estimate counts the tree, and the tree shares.** A node is
  shared between the document and the history's copies of it, and the
  estimate counts it once per copy. So the history's budget is
  **conservative**: two states of the same document cost what two
  documents cost. That is the right way to be wrong.
- **The plan is not bounded, and does not need to be.** The plan is
  **derived** on demand and handed to the caller; the session does not
  hold it. A plugin's value may be any size — and that is the
  plugin's business, and the plan is not a leak.
- **The bytes are an estimate, and the allocator is the truth.** The
  estimate never under-estimates and it does not pretend to be the
  allocator's number: a program's own allocator may hold more, and a
  program's own bound is the way to say so.

---

## 66. One budget for the whole session, and the order things go in

§65 bounded each part of a session on its own: the document, the
history, the journal, the tasks, the log. This section is about the
**whole** — and it closed §63's unsettled line, which said that a
document *built* in memory was bounded only when it was written.

### What was built

- **The session judges the document after every command.** A command
  that leaves the document over its byte budget is **refused and put
  back**: the document is restored, the command reports the refusal, and
  the work that was there before is still there. A document nobody could
  save is a document nobody should have.
- **The session has a budget of its own.** `MAX_SESSION_BYTES` is the
  **family's** name and the **family's** number: the document's budget
  times the number of documents a session holds — two of its own, one
  snapshot and the history's two — which is one hundred and sixty
  mebibytes.
- **The session says what it is holding, and it says the parts.**
  `Session::memory()` gives the document, the history, the journal, the
  tasks and the log separately, because a session that is over its
  budget has to give *something* up and the part that gives way is the
  part that is cheapest to lose.
- **The order of what gives way** is the same in every session:
  1. the **journal** — a record of what was asked for, and the newest
     entries are the useful ones;
  2. the **finished tasks** — their results have been collected, or
     nobody is waiting for them; a task that is still **running** is
     never dropped;
  3. the **history** — the oldest states, which are the ones a person
     has already undone past.
- **The document is not in the order.** It is never given up: it is the
  work.
- **The measurement is a command.** `session.memory` says what the
  session is holding and what it may hold, because everything a person
  or an agent can see is a command in this kit — and a budget nobody can
  look up is a budget nobody can check.

### What the port found

- **§63's unsettled line is closed.** The depth bound was checked while
  the node was read and the count bound when the document was read or
  written — and a document assembled **in memory** was bounded only
  when it was saved. The session now judges it after every command, and
  puts it back. The bound bites where the work happens, not where the
  file is written.
- **A budget over parts is a budget with an order.** The parts cannot
  all give way at once, and which one goes first is a **decision** and
  not an implementation detail: the journal is a record of what was
  asked for, the finished tasks are work nobody is waiting for, and the
  history is the work itself. A program that lost its undo steps first
  would be a program that lost the session.
- **The document is never in the order.** A budget may cost the
  history, the journal and the tasks; it may not cost the document. The
  same rule the family keeps for the state that is current in the
  history: a bound that drops the working document is not a bound, it
  is a data loss.
- **The parts' budgets add up to less than the session's budget.** The
  document is thirty-two mebibytes, the history sixty-four, the journal
  and the tasks eight each — and the session's budget is one hundred
  and sixty. So with the kit's own numbers the session budget **does
  not bind**, and it binds exactly when a program raises one of the
  parts. That is the point of having both: the parts are the parts'
  business, and the whole is the program's.
- **The number that says what the process uses is not the number that
  says what one document holds.** The family learned this the hard way
  (§144): a table that claimed one document's budget was wrong by a
  factor of four, because the session holds the document, the snapshot
  and the stacks. Two trees now say the same thing about a session.

### What it cost

- two constants and the derivation, one struct (`Memory`), three
  methods on the session, one method on the history, one on the task
  registry, and one command;
- **eight tests** in a new file (`tests/memory.rs`), beside the ten
  that hold §65's bounds.

### What it does not settle

- **The budget is measured, and the allocator is the truth.** The
  estimate counts the parts and never under-estimates; it does not
  count the plugins' own memory, the process's stack, or the allocator's
  overhead. The number says what the **session** holds, not what the
  process uses.
- **Giving way is not undoing.** When the journal gives up its oldest
  entries and the history gives up its oldest states, they are **gone**:
  a person who wanted them is out of luck. A bound costs something, and
  the cost is named here rather than discovered at the worst moment.
- **The document is not bounded while the command runs.** The session
  judges the document **after** the command, so a command that builds a
  very large document holds it for the length of that command. The
  bound is on what the session *keeps*.
- **The memory is not measured between commands.** The budget is checked
  after each command that ran; a program that fills the session from
  outside the session — a plugin that holds its own copies — is not
  counted.

---

## 67. The canvas, and a picture of the document

The kit has had a **painting seam** since the beginning: every kind of
node knows how it is painted, and the core never matches on a kind. What
it did not have was a canvas to paint on — the only backend was a
recorder that wrote down what it was asked to draw, which is enough to
*test* a kind and not enough to *see* one.

Every application in the family draws. So this is the piece that turns
the kit from a document model into something a person can look at.

### What was built

- **`craftkit-raster`** — a **library**, not a plugin: it contributes
  nothing to the registry and the linker does not wire it in.
  - a **picture**: a buffer of RGBA pixels with a background;
  - a **canvas**: the kit's own `Canvas` trait, implemented over that
    buffer, with the **camera's** zoom and offset applied — one set of
    camera controls, and the canvas goes through them;
  - a **PNG writer**: the picture as a file anybody can open, with no
    dependency and no compression (the deflate stream is *stored*).
- **The command `doc.snapshot`** — the application takes a picture of
  the document and writes it to a file. It knows nothing about what a
  rectangle is: the canvas is given the drawing, and the drawing comes
  from the kind.
- **The picture is bounded.** `MAX_IMAGE_PIXELS` (sixteen million
  pixels) and `MAX_IMAGE_SIDE` (eight thousand and ninety-two) are the
  crate's own numbers and they are named here. A picture that is asked
  to be enormous is **refused** and not made smaller.

### What the port found

- **The seam was real, and nothing exercised it.** Every battery
  declares how its kinds are painted, and no picture had ever come out
  of it — so a painting bug could not have been found, because there was
  nothing to look at. The seam was a promise with no witness.
- **The camera was already there, and the canvas had to go through it.**
  The camera is a zoom and an offset (§ the camera's own section), and
  the rule is that every canvas goes through the one set of controls. The
  rasteriser takes the camera and applies it: the world's **up** is the
  picture's **down**, because a document is drawn the way it is looked
  at and not the way it is stored.
- **The canvas has no font of its own — and that is answered in §540.**
  The kit has its own typeface (§128), so the canvas draws **letters**
  now: a character is a stroke and not the box it would occupy. The
  cell is still one for every character, so a line of text is still
  monospaced; what changed is that the cell holds a letter. A battery
  draws its own letters as paths (§129), and the two agree.
- **A picture without soft edges is a picture a machine can reason
  about.** There is no anti-aliasing: a pixel is either painted or it is
  not, and the fill is a scanline over the flattened curves. That is a
  choice and not an omission — an edge that is exactly where it was put
  is an edge that can be checked.
- **The PNG writer is a hundred lines and no dependency.** The whole
  point of the kit is that it has no dependency it did not choose, and a
  PNG is a signature, three chunks, a CRC and a stored deflate stream.
  The test **reads the file back** — every chunk, every CRC, and the
  picture out of the stream — because a writer that is only trusted to
  be right about itself is not a writer.
- **The picture is not an edit.** `doc.snapshot` does not change the
  document, so it makes no undo step: a photograph is not a change.

### What it cost

- one crate, one file's worth of PNG arithmetic, and one canvas;
- one command in the application and three tests beside it;
- **nine tests** in the crate, and **twelve** in all.

### What it does not settle

- **The picture is not the document.** It is what the kinds drew, and a
  kind that draws nothing draws nothing. There is no second rendering
  path and no "what the document really looks like" — there is only what
  the kinds say, which is the point of the seam.
- **There is no anti-aliasing and no soft edges.** A diagonal line is a
  staircase, and a circle is a polygon. A picture for a person to look
  at would want both, and the choice here was a machine-readable
  picture.
- **The canvas's text was a box and not a letter.** The boxes were a
  placeholder that said where text would be; the kit's own typeface
  (§128) is a table of strokes and the rasteriser draws them now
  (§540). The seam did not change: the canvas still takes a string and
  a point.
- **The canvas is a raster, and not a display list.** Everything is
  painted into pixels as it happens; a program that wanted to zoom in
  after the fact would have to paint again. The camera is applied while
  drawing, and that is the only place it is applied.

---

## 68. A picture is a size and a file

The kit had no idea what a picture was. An image was "a plugin's
payload" — a thing an application knew about and the kit did not — and a
document that held a picture had nowhere to put it but the document
itself. That is the wrong shape, and this section is where the shape is
written down.

### What was built

- **`craftkit-image`** — a **library**, not a plugin: it reads the
  **size** of a picture and nothing else.
  - **JPEG** — the frame header, walked the way ITU-T T.81 lays the
    markers out. The walk is ported from the family's own clean-room
    decoder (`craft-jpeg`), which is the tree that knows about the
    twelve-bit and the arithmetic-coded files that the common decoders
    refuse;
  - **PNG** — the `IHDR` chunk, which is the first thing after the
    signature and says the size in four bytes each way.
- **`craftkit-plugin-image`** — the **thirteenth battery**: a **picture**
  is a node whose **payload** holds the size and the format, and whose
  **stream** is the file. The commands are `image.add` and
  `image.measure`, and the aspect ratio and the pixel count are
  **derived**.
- **The head of a file is read, and the rest is not.**
  `craftkit_sdk::files::read_prefix_bounded` reads what it was asked
  for and stops: a picture of forty megabytes is measured by reading a
  few kilobytes of it. The bounds this section adds are the kit's own
  and are named here: `MAX_HEADER_BYTES` (a megabyte of header) in the
  size reader, and the battery's own head (a megabyte) beside it.

### What the port found

- **A picture is a size and a file, and never the pixels.** The family
  keeps the bytes beside the document and writes them into the package
  on save — so the document an application edits holds two numbers and
  a name, and the file holds the rest. That is the kit's **stream**
  (§25) arriving at the pictures, and it is the third tree to say it
  after the artist's stage and the prose's media.
- **The header is the only part of the picture the document needs.** So
  the reader reads the **head** and stops. The measure of a picture is
  cheap in a way the measure of a document is not: a document has to be
  read to be known, and a picture does not.
- **A picture is an *open* kind, and the format is data.** The family
  has a vector of picture kinds — raster, vector, PDF — and every new
  one is a change in the model, the reader, the writer and the panel.
  Here the payload says which format it is, and the kit says nothing
  about what a raster is. The size reader knows two formats and refuses
  the rest with a **name**, not with a panic.
- **A picture that cannot be measured is not a broken picture.** The
  bytes may be the tail of a file, or a file whose header is further in
  than the reader was told to look. The refusal says what was wrong and
  stops.

### What it cost

- one library with six tests, one battery with four tests beside it, and
  one method on the file reader;
- **no decoder**: there are no pixels here, and a decoder is a library
  of its own.

### What it does not settle

- **The pixels are still not in the kit.** The size reader measures a
  picture; it does not decode one. The family's clean-room decoder is a
  thousand lines of Huffman and IDCT and arithmetic coding, and the
  question of where it belongs — a library beside this one, or a plugin
  that draws — is a question this section does not answer.
- **A picture is not drawn.** The canvas (§67) draws rectangles and
  paths and the boxes that text would occupy; it cannot put a picture
  on the page, because the pixels are in a file and the canvas is a
  buffer.
- **The head that is read is a limit and not a promise.** A picture
  whose header is further in than the limit is a picture this program
  cannot measure — and the program chose the limit.

---

## 69. The one way, and the second path that was still open

The family tree has a rule that is not about the code but about what the
code **is for**: the session's public methods are the registry, and a
public method that changed a design would be a **second path** to it. The
gate in the family refuses one (§135).

This tree had the same hole and did not know it. `Session::document_mut()`
was public and handed out the document to anybody who asked — including a
program that had no command running, and therefore no snapshot, no undo
step and nothing in the log. The batteries used it, the tests used it, and
every use of it was a way around the registry.

### What was built

- **The document is handed out only while a command is running.**
  `Session::document_mut()` returns a `Result` now, and it is a refusal
  when no command is running: *"the document is changed by a command, and
  no command is running"*. The session already knew whether a command was
  running — it keeps the name for the crash report — so the rule costs
  nothing to enforce.
- **Every call site went through the registry.** A hundred and sixteen of
  them, in twenty-seven files: the batteries, the SDK's own commands and
  the tests.
- **The tests that built documents by hand now build them the way the
  reader sees them.** A test that needs a metric with two thousand
  widths — a thing no command would ever make — now hands the **reader**
  the node it is to read, and the reader is what is being tested.
- **The command the rule asked for exists.** Changing a metric's widths
  was something a program could do by hand and not through the registry,
  so there was no command for it; now there is: `text.metric.change`.
- **A new file for the rule** (`crates/craftkit-sdk/tests/one_way.rs`),
  with three tests: the refusal outside a command, the handover inside
  one, and the judgement that follows.

### What the port found

- **A rule that is not enforced is a hope, and this one had been
  enforced for months by nobody.** The hole was not in the batteries: it
  was in the *session's own API*, and every battery had copied the shape
  of it. The family found the same hole in its own tree and put a gate
  in; the gate was the right answer there, and here the *runtime* is
  better: the session refuses, so there is nothing to forget to call.
- **Closing the second path found a missing command.** "A typeface's
  widths change" was a thing the kit could do and could not *say*: the
  only way to change a metric was to reach into the document. The rule
  did not only close something — it **named a gap**, and the gap was a
  command that should have existed.
- **The tests that were cheating were testing the wrong thing.** Three
  of them built documents that no file and no command could produce, and
  then asked the *session* about them. They are now asking the *reader*,
  which is where the question belonged, and they are better tests for
  it: a reader that survives a metric with two thousand widths is a
  reader that survives a file that has one.
- **A refusal is a value, not a crash.** A program that reaches for the
  document outside a command gets an error it can read and handle, and
  the document is untouched.

### What it cost

- one signature, one guard, a hundred and sixteen call sites, one new
  command, and three new tests;
- the shape of the batteries' commands did not change: a command still
  receives the session and writes through it.

### What it does not settle

- **The session still hands out the document to a command that lies.**
  A command that says it does not change the document and changes it
  anyway is caught afterwards — the session compares the document with
  the snapshot and puts it back (§ the judgement) — but it *is* caught
  afterwards and not before.
- **The rule is the session's, and not the gate's.** The family puts a
  gate in front of the same hole; here the session refuses, which is
  stronger at run time and weaker at build time. A crate that reaches
  for the document is refused **when it runs**, and not when it is
  compiled.
- **`set_document` is the door the plugin's bridge uses.** It is the
  session's own business and not a command: a program hands a document to
  a session that has none, and nothing is undone because there was no
  edit. It is `pub(crate)`, so it is not a second path out of the crate.

---

## 70. The pixels

§68 left one line open: *the pixels are still not in the kit*. This
section closes it — with the **family's own decoder**, ported whole.

### What was built

- **`craftkit-jpeg`** — the family's clean-room decoder for the JPEG
  processes most decoders do not handle, written from ITU-T T.81:
  sequential DCT with Huffman coding at eight and twelve bits, extended
  sequential with **arithmetic** coding, and progressive with arithmetic
  coding. It is ported **whole** — thirteen hundred lines, the tables,
  the IDCT, the arithmetic decoder and the colour conversion — and the
  port changed two things:
  - the **error type** says its three cases out loud instead of deriving
    them from `thiserror`, because this kit has no dependency it did
    not choose; and
  - the crate root carries this tree's **deny list** and the
    `forbid(unsafe_code)`, and the decoder's own code needed no change
    for either: it has never had an `unwrap`, an `expect` or a `panic`
    in it.
- **The canvas draws a picture.** `Raster::blit` takes the decoder's
  samples — interleaved by pixel, the rows top to bottom — and scales
  them into a rectangle, with the **nearest** pixel and no filtering.
- **A picture has a place.** `image.add` takes a rectangle, and the
  picture node holds it.
- **The snapshot draws the pictures in the document.** `doc.snapshot`
  takes the folder the streams are relative to, decodes every picture it
  can and draws it where the node says. A picture that cannot be decoded
  is **reported and not drawn**: the snapshot is of the document, and a
  broken picture is not a reason to refuse the whole of it.
- **The tests are on a picture written by hand.** There is no encoder
  here and no dependency: the test builds a baseline JPEG byte by byte —
  the quantisation table, the frame, the two smallest Huffman tables
  there are, and the entropy coded data — and the decoder reads it back
  as a mid-grey block.

### What the port found

- **The family's decoder is already a kit citizen.** It was written for
  a tree with the same rules — no panics on input, the limits are the
  caller's, the allocations are checked before they are made — and it
  dropped into this tree with **one** change. That is the strongest
  evidence yet that the rules are not this tree's taste: they are
  **portable**, and a piece of code written under them lives under
  another set unchanged.
- **A decoder and a document are different kinds of thing.** The
  decoder's output is pixels, and pixels do not go in a document: they
  are big, they are binary, and they belong to nobody but the file they
  came from. So the decoder is a **library** and the picture is a
  **node** whose payload is two numbers and whose stream is the file.
  The kit never holds the pixels — it *draws* them and lets them go.
- **The scale of a picture is the front end's business.** The canvas
  scales by taking the nearest pixel, and it says so; a canvas that
  filtered would be a canvas with an opinion about pictures, and this
  one has none.
- **A picture that cannot be decoded does not stop the picture.** The
  snapshot reports what it could not draw and draws the rest. A
  document with one broken picture is still a document.

### What it cost

- one crate of thirteen hundred lines, ported and adapted in two places;
- one method on the canvas, one parameter on a command, and one loop in
  the snapshot;
- **five tests** on a hand-built picture and **one** on the canvas.

### What it does not settle

- **PNG is written and not read.** The kit writes PNG (§67) and reads
  its *header* (§68), but it does not decode one: a PNG reader needs an
  **inflate**, and an inflate is a library of its own. The snapshot
  draws JPEG and reports the PNG as what it cannot read.
- **The decoder is JPEG only.** The family has no PNG decoder either,
  so this is not a gap the port opened — it is a gap the family has and
  the kit inherited.
- **The decoder is the family's, not this tree's.** It was written for
  another tree, under another crate's name, with another tree's
  comments; the port changed the error type and the lint list and
  nothing else. What is *not* settled is whether the kit should own the
  decoder or merely borrow it: a decoder is a library, and a library
  that is borrowed is a library that can be replaced.

---

## 71. The inflate, and the PNG the kit could write but not read

§70 left a line open: *PNG is written and not read*, because a PNG's
pixels are in a **zlib** stream and the kit had no inflate. This section
is the inflate.

### What was built

- **`craftkit-png`** — a **library**, and it depends on nothing:
  - the **chunks**: the signature, the header, the palette, the data
    and the end, each with its **CRC checked** — a picture whose own
    word about itself does not match is a broken picture;
  - the **inflate** (`inflate.rs`): RFC 1951's three kinds of block —
    stored, fixed Huffman and dynamic Huffman — and the zlib wrapper
    around them, with the Adler-32 checked at the end;
  - the **filters**: PNG's five ways of storing a row, undone;
  - the **pixels**: the samples, as they are;
  - the **transparency**: the `tRNS` chunk, which says which of a
    picture's colours are see-through, and which comes out as an alpha
    beside every pixel (§77).
- **The snapshot reads PNG as well as JPEG**, and draws it.
- **The tests are on pictures made by two writers**: the kit's own,
  which *stores* its data and does not compress it, and the `image`
  crate's, which compresses it — and a compressed picture is what
  exercises the Huffman paths.

### What the port found

- **The canonical order is the whole of a Huffman decoder.** The codes
  are numbered by length first and by symbol second, and the table's
  symbols sit in that order. Getting the *offsets* wrong — starting
  each length's symbols after the previous length's, instead of after
  every **shorter** length's — makes a decoder that reads valid
  streams as invalid ones, and it does so **quietly**: the table it
  builds is over-subscribed, and the over-subscription is the only
  symptom. The bug was found by a picture that the `image` crate wrote
  and the kit could not read, and it was found by *reading the bytes*,
  not by reading the code: the code looked like puff's, and puff's is
  right.
- **A decoder is tested against a compressor, not against itself.** The
  kit's own writer stores its data, so it exercises only the stored
  path; the Huffman paths are exercised by streams somebody else
  compressed. The tests borrow two compressors — `flate2` and
  `image` — and they are **test-only** dependencies: nothing that
  ships here depends on either.
- **The zlib wrapper is part of the format, not part of the decoder.**
  The two header bytes, the check bits and the trailing checksum are
  read and checked, and a stream with a preset dictionary is a refusal
  with a name.
- **The interlaced picture is a refusal, not a half-picture.** Adam7 is
  a different shape of file, and the reader says so instead of drawing
  the wrong thing.

### What it cost

- one crate of about six hundred lines, in two files: the inflate and
  the reader;
- **ten tests**: six on pictures and four on streams;
- two test-only dependencies, and no dependency that ships.

### What it does not settle

- **The bit depths of one, two and four bits are not read.** Eight and
  sixteen are; the others are a refusal with a name.
- **A sixteen-bit picture comes out as eight-bit.** The high byte of
  each sample is taken, because that is what a canvas wants and what a
  document holds — and the picture the kit draws is not the picture the
  file carried, in the way a picture shown on a screen is not the
  picture a camera took.
- **The kit's writer does not compress.** A PNG the kit writes is
  bigger than it needs to be, because the writer has no compressor and
  the compressor is a library of its own. The reader has one now: the
  inflate. The writer may yet use it.

---

## 73. The numbers agree with the prose

§72 closed the half of the rule that says a bound must be **named** in
the prose. This section closes the other half: the prose must say the
**number**, and the number must be the number the code has.

### What was built

- **A table of the bounds**, in this README, with the expression each
  one is written with:

| the bound | the number |
|---|---|
| `MAX_DOCUMENT_BYTES` | `32 * 1024 * 1024` |
| `MAX_HISTORY_BYTES` | `MAX_DOCUMENT_BYTES * HISTORY_EXPANSION` |
| `MAX_SESSION_BYTES` | `MAX_DOCUMENT_BYTES * SESSION_DOCUMENTS` |

- **A small evaluator** (`crates/craftkit-gates/src/numbers.rs`): the
  four operators an arithmetic bound is written with — `*`, `+`, `-` and
  `<<` — and the tree's own constants as the names it may use. The two
  expressions are **evaluated** and compared, so `32 * 1024 * 1024` and
  `33554432` are the same number, and a reader cannot tell that from
  the text.
- **The fifth rule of the gate**: a bound whose number and the prose's
  number differ is a finding.

### What the port found

- **The prose was telling the truth in words and the code was telling
  it in numbers, and nothing compared them.** §63 said the document's
  budget was thirty-two mebibytes; §65 said the history's was two
  documents; §66 said the session's was five. The words were right and
  the numbers were right, and a change to one would not have touched
  the other. That is the drift the outer tree's own rule exists to
  stop (§144: two caps with the same value and no relation between them
  drift apart the moment one of them is changed).
- **A number that cannot be worked out is not checked, and the gate
  says nothing.** The evaluator knows four operators and a name; a bound
  written any other way is a bound the gate **cannot** check, and a
  gate that guesses is worse than a gate that is quiet.

### What it cost

- one module of about two hundred lines, one rule, and the table above.

### What it does not settle

- **The table is this README and nothing else.** A bound named in a
  recipe and not in the table is a bound the gate cannot check, and the
  rule says the bound is *named* (it is) while the number goes
  unchecked.
- **The evaluator is small on purpose.** A bound written with a cast, a
  function call or a shift by a variable is not checked, and the
  numbers this tree uses are all arithmetic.
- **The gate checks the tree's own constants, not the compiler's.** A
  constant that the compiler would fold differently — the evaluator
  works in `u64` and the code in `usize` — is the same number on a
  64-bit target and not on a 32-bit one. The tree is 64-bit.

---

## 72. The fourth rule: the bounds are named in the prose

The outer tree has a rule that is about **prose** and not about code:
every public bound in the tree is named in a document, because a bound a
reader cannot look up is a bound a reader cannot check. It caught this
tree's own work in §414 — the size reader's byte cap was public and no
document named it — and the fix was to make the cap private and name it
here.

That is a good rule, and this tree now has it as **a rule of its own**:
the fourth rule of `craftkit-gates`, checked by a program, over this
tree, against this tree's README.

### What was built

- **The rule.** Every **public** `MAX_…BYTES`, `MAX_…MEMORY` or
  `MAX_…STEPS` constant in the tree is named in the README. A name is
  named when it is in the prose as a **word** — a bound that is a prefix
  of another bound's name is not named.
- **The rule is a function over a root**, like the other three: the
  gate takes a tree and says what it found, so a test can point it at a
  tree broken on purpose.
- **Three tests that say the rule bites**: a public bound with no
  document naming it is caught; a bound the prose *does* name is not; a
  **private** bound is not caught either, because a private cap is the
  program's own and the rule is about the promises the tree makes.

### What the port found

- **The rule the outer tree had was the rule this tree needed.** §414
  was the outer gate reaching into this tree and finding a bound that
  no document named — and the fix was to name it *here*, in a README
  the outer gate does not read. That is a rule that is **enforced in
  the wrong place**: the number is in this tree's code and the reason
  for it was in another tree's prose. A rule that is enforced elsewhere
  is a rule that is enforced *late*.
- **A bound that no document names is a promise nobody can rely on.**
  The public bounds of this tree are three — the document's, the
  history's and the session's — and every one of them is named here
  with the number and the reason. That is the whole of the rule, and
  the whole of it fits in a paragraph.
- **The rule checks the names and not the numbers.** A bound named in
  the prose with the *wrong* number in it is not caught: the prose says
  what the bound is for, and the code says what it is. That is a gap,
  and it is named here rather than left to be found.

### What it cost

- one rule, about a hundred lines, and three tests.

### What it does not settle

- **The numbers are not checked against the prose.** The rule says the
  bound is named; it does not say the prose and the code agree about
  its value. The outer tree's own rule has the same shape.
- **Only the public bounds are checked.** The private caps — the
  journal's, the tasks', the header's — are named in this README too,
  but the gate does not ask for it, and a private cap is not a promise
  the tree makes to anybody but itself.
- **The prose is this README and nothing else.** The other documents of
  this tree — the recipes, the architecture notes — are not read by the
  gate, and a bound named only in one of them is a bound the gate
  cannot see.

---

## 74. The depths below eight

§71 read the PNGs it could and refused the rest. This section reads the
rest: the **one-bit**, **two-bit** and **four-bit** pictures, which are
the ones a picture has when it is mostly one colour.

### What was built

- **The samples of a picture of fewer than eight bits are unpacked.**
  They are packed into the bytes, most significant bits first, and a
  grey picture's values are scaled up: a one-bit grey is black or white,
  a four-bit grey is one of sixteen greys, and the picture that comes
  out is eight-bit.
- **A palette picture's samples are the palette's indices**, and the
  picture that comes out has **three** samples a pixel — the colours,
  and not the indices. The reader says which it is, because a caller
  that reads one grey where there are three colours draws nonsense.
- **The filters count a packed pixel as one byte**, which is what PNG
  says: the byte to the left of a packed pixel is the byte before it,
  and not the pixel before that.

### What the port found

- **A row's bytes and a pixel's bytes are different numbers.** The row
  of a four-bit picture holds as many bytes as its samples need — four
  pixels are two bytes — while the *filter* counts a pixel as one byte,
  because that is the unit the filter works in. Getting that wrong
  makes the reader say the picture's rows are shorter than they are,
  which is a refusal and not a wrong picture: the reader was right to
  refuse and wrong about why.
- **The picture that comes out is not the picture that went in.** A
  four-bit grey comes out eight-bit and a palette comes out as
  colours, and a caller that does not know that reads nonsense. So the
  reader says what it produced — how many samples a pixel has — and
  the rule is that the *samples* are what a picture is, and the file's
  encoding of them is the file's business.

### What it cost

- one function, and two tests on pictures written by hand.

### What it does not settle

- **A sixteen-bit picture comes out eight-bit**, and a one-bit picture
  comes out eight-bit too: the kit's pictures are eight-bit, because
  that is what a canvas wants. The values are scaled, and the scaling
  is the file's convention (a four-bit grey is multiplied by seventeen),
  not the kit's.
- **The transparency of a picture is not read.** A `tRNS` chunk says
  which colours are see-through, and the reader ignores it. (This was
  settled later, in §77: the chunk is read, and a picture with a
  transparency comes out with an alpha beside every pixel.)
- **Interlaced pictures are still refused** (§71), and that is the one
  shape of PNG this reader does not take.

---

## 75. The terminal door

The kit had three doors for other programs — the MCP server, the
control channel and the plugin bridge — and **none of them was for a
person**. A person cannot speak MCP, and the prompt is the oldest and
the simplest of the kit's doors: it is a line of text, and the kit's
own vocabulary is a line of text.

### What was built

- **`apps/craftkit-repl`** — a program that asks a question, reads a
  line, and answers it. The built-ins are `help`, `commands`, `plan`,
  `memory`, `snapshot <path>` and `quit`; everything else is a command
  of the kit's own, written `id` or `id {"name": "value"}`.
- **The door reaches the registry and nothing else.** `plan` is
  `doc.plan`, `memory` is `session.memory`, and `snapshot` is
  `doc.snapshot` — the door asks, and it prints. There is no second
  way to take a picture or to read a plan, so the answer a person gets
  at the prompt is the answer the MCP server gives the agent.
- **`doc.plan` is a new command** (§74 closed the PNG reader; this
  closes the gap the plan left). The studio, the bridge and the
  terminal door all wanted the plan, and each of them reached for the
  evaluator itself. So the plan is a command now, and the three of
  them ask it the same way.
- **The line is bounded, and the door owns the bound.** A person may
  paste a whole file into a prompt, so the line is read within a limit
  and a line over it is refused rather than read. The limit is
  `MAX_LINE_BYTES` — sixty-four kibibytes — and it is **private**: it
  is this program's own number, and a public bound is a promise
  anybody else may rely on, and this one is not a promise to anybody.
  The reader that enforces it is the kit's own
  (`craftkit_sdk::lines::read_line_bounded`, §62): the mechanism is
  shared and the number is the caller's (§17).

### What the port found

- **A door with built-ins is a second registry, unless the built-ins
  are commands.** The first draft of the terminal door drew the
  picture itself, read the plan itself and asked the session for its
  memory itself. Every one of those was a second path to an answer
  the kit already had a command for, and the second path is where two
  answers drift apart. The door now asks the command and prints it.
- **The plan was the one thing the kit could not say out loud.** The
  registry was the front ends' only source (§4), and the plan was the
  exception: it was the one answer a front end had to compute for
  itself. That is what `doc.plan` is for, and the finding is the
  general one — *if two front ends both want it, it is a command*.
- **The command's own log entry changes the number it reports.** A
  command writes a line in the session's log, so the log is one line
  bigger after a measurement than it was during it. The door shows the
  state as it was when the command ran, and the log says the command
  ran. This is not a bug to be smoothed over: it is the difference
  between "how much is held" and "how much is held, and what was done
  to it".

### What it cost

- one application, one command in the SDK, and **thirteen tests** on
  the door.

### What it does not settle

- **The prompt is a prompt and not a shell.** There is no editing of
  the line, no completion, and no history. A person who wants those
  wants a shell, and a shell is a program that talks to this one.
- **The door is one ecosystem.** It drives the studio's ecosystem, so
  the batteries it offers are the studio's — the optional ones are off
  unless the studio is built with them.

---

## 76. The CLAP door

The kit had a door for its **own** plugin format: a shared library, an
entry point found by name, a versioned table (§34). The real formats —
CLAP, VST3, AU — were "not built", and the reason given was that each
is a large ABI in another language. CLAP is the one of the three whose
ABI is **published and open**, so it is the one that could be written
down from a specification rather than guessed at. (VST3's is published
too, and it came four arcs later: §116–§120.)

### What was built

- **`crates/craftkit-clap-abi`** — the **shapes**: the version, the
  entry, the factory and the descriptor, and the two names a host looks
  for (`clap_entry` and `clap.plugin-factory`). It has **no
  dependencies** and **no `unsafe` at all**: a binary contract is a set
  of shapes, and a set of shapes is data.
- **`crates/craftkit-clap`** — the **host**: opens a shared library,
  finds the entry, checks the version, initialises the plugin, asks for
  the factory by its id, and reads the descriptors out of it. It is the
  **fourth** crate that may hold `unsafe`, and it is held to the same
  rules as the others: `deny(unsafe_code)` and not `forbid`, **one**
  `allow(unsafe_code)`, and one marked module where every unsafe block
  says why it is sound.
- **`crates/craftkit-clap-fixture`** — a **reference CLAP plugin**: a
  real `cdylib`, built by cargo, exporting the entry by name. It is the
  **fifth** crate that may hold `unsafe`, and the only unsafe in it is
  the attribute that gives the symbol its name.

### What the port found

- **A format's entry point may be a data symbol.** Craftkit's own door
  finds a **function** and calls it. CLAP's entry is a **variable**:
  `clap_entry` is a pointer whose value is the address of the entry
  struct. So the host reads the pointer *at* the symbol's address, and
  then the struct *at* that pointer — two reads, and the other door
  cannot be reused, because it was written for a function.
- **The unsafe is on the two sides of the contract, and not in it.** The
  shapes live in their own crate and hold none; the host reads the
  plugin's memory; the plugin exports the symbol. The list of
  exceptions grew from three to five, and each new entry has a reason
  beside it — which is the shape §34 found, held again.
- **A plugin's strings are copied out at once.** A descriptor's strings
  belong to the plugin, and a string read later may be gone by then, so
  every string is read and copied in one go — and read **bounded**:
  `MAX_STRING_BYTES` is sixty-four kibibytes and `MAX_FEATURES` is two
  hundred and fifty-six. Both are **private**: they are this host's own
  numbers, and a public bound is a promise anybody else may rely on.
- **A static may not hold a pointer.** The fixture's descriptor is a
  struct of pointers, and a static is shared, and a raw pointer says
  nothing about whether it may be shared. So the descriptor is built
  once on the heap and its address is kept in an **atomic pointer** — a
  pointer that may only be read is a pointer that may be shared.
- **The version is checked before anything is called.** The rule is the
  format's own: the major versions must agree. A plugin of another
  major version is **refused**, not used, because the shapes may have
  changed.

### What it cost

- two crates and a fixture, and **seventeen tests** — six on the shapes
  (including the **layout**: the offset of every field, because a field
  that moved is a field the host reads in the wrong place) and eleven
  on the host.

### What it does not settle

- **The host reads a plugin; it does not make one.** Making a plugin
  needs the structure the format calls the **host** — the thing the
  plugin calls back into — and this kit has no such structure yet. So
  there is no audio path here: no processing, no ports, no events.
- **VST3 and AU are still not here.** CLAP was the one with an open,
  published C ABI that could be written down from its specification.
  The other two are larger, and each would be its own arc. (VST3's was
  its own arc, four sections later: §116–§120. **AU** is still to
  come.)
- **The fixture does nothing.** It holds one plugin, and its
  `create_plugin` answers with nothing. A host that has read a plugin
  has not yet used one, and the difference is the whole of the next
  arc.

---

## 77. The picture's transparency

The kit could write a PNG and read a PNG's pixels, and the one thing
the reader threw away was the `tRNS` chunk — the part of the file
that says **which of the picture's colours are see-through**. A
picture with a transparency came out as if it had none, so a picture
with a see-through background came out with an opaque one, and
nothing in the kit could tell the difference.

### What was built

- **The `tRNS` chunk is read**, and what it means is decided by the
  picture's **colour type**: one grey value for a grey picture, one
  colour for a colour picture, and an alpha for each of a palette's
  entries for a palette picture.
- **A picture with a transparency comes out with an alpha.** The
  samples grow by one — a grey becomes grey and alpha, a colour
  becomes red, green, blue and alpha, and a palette becomes colours
  and alpha — and the picture says how many samples a pixel has, so a
  caller can tell.
- **The chunk is only allowed where the format allows it.** A grey or
  a colour picture of fewer than eight bits may not have one, a
  picture that already has an alpha channel may not have one either,
  and a transparency that names more palette entries than the palette
  has is refused. Each of those is a refusal **with a name**.
- **The reader was split in two.** The part that undoes the filters
  now hands back the bytes as they are stored, and a second part
  **composes** the picture: the palette's indices become colours, a
  grey of fewer than eight bits is scaled up, a sixteen-bit picture
  becomes eight-bit, and the transparency becomes an alpha.

### What the port found

- **A transparency is not a colour.** The chunk does not say "this
  pixel is see-through"; it says "this **value** is see-through", and
  the picture decides which pixels have that value. So the transparency
  has to be read **before** the picture is composed, and a sixteen-bit
  picture has to be compared at sixteen bits: a grey of `0x0100` is
  not a grey of `0x01ff`, even though they share the high byte the
  picture ends up with. The first draft compared the eight-bit
  samples and would have made the wrong pixels see-through.
- **A picture's channels are what the picture is, and the file's
  encoding is the file's business.** A palette picture has one sample
  a pixel in the file and four in the kit; a picture with an alpha
  channel in the file has four and keeps them. The number the reader
  reports is the number of samples in the **kit's** picture, and that
  is the number a canvas wants.
- **A chunk's meaning can depend on a chunk that came before it.** The
  `tRNS` chunk is two bytes for a grey picture and six for a colour
  picture, and a palette's transparency may be two bytes as well — so
  the length alone does not say what the chunk means. The reader
  keeps the bytes and reads them when it knows the colour type.

### What it cost

- one chunk read, one step of composition, and **nine tests** on
  pictures written by hand — because no writer writes a `tRNS` chunk
  on purpose.

### What it does not settle

- **A picture with a transparency is a picture with an alpha**, and
  the alpha is **binary**: see-through or not. The format says a
  palette's entries may be partly see-through, and that is read; but
  a grey or a colour picture's transparency names one value, and that
  value is either seen or not.
- **The kit's own writer still writes no transparency.** It writes
  RGBA, which is the whole of the alpha channel, so it has no use for
  a `tRNS` chunk — and a reader that reads transparency and a writer
  that cannot write it is a half of a round trip.

---

## 78. The browser door

The kit had doors for other programs and one for a person at a
terminal. A browser is the other place a person is — and the **old
tree's** web app cannot be built on this machine: the
`wasm-bindgen` command-line tool here is 0.2.89, the crate is 0.2.129,
and the two have to match exactly (`KNOWN-GAPS.md` §13).

So the question this arc asked was not "how do we use the bundler" but
**"does the door need one at all"**.

### What was built

- **`apps/craftkit-wasm`** — the **browser door**: a WebAssembly module
  and the page that loads it.
- **There is no bundler in it.** The module is the file cargo built,
  and the page loads it with `WebAssembly.instantiate`. Nothing has to
  be installed to open the kit in a browser, and the old tree's
  blocker does not apply: there is no `wasm-bindgen` in this tree at
  all.
- **The boundary is four exports and two buffers**: the room for a
  command and its address, the run, the answer's address and its
  length. A page writes the command into the module's memory and reads
  the answer out of it.
- **The door is the terminal door.** It runs the same `Repl`, against
  the same registry, and a test says that the page and the prompt say
  the same thing for the same command.
- **The bundle** is a program: it writes the page and the module side
  by side into `dist/`, and a person serves that directory with
  whatever they have.
- **The command is bounded.** A page can be handed anything at all, so
  a command over `MAX_COMMAND_BYTES` — sixty-four kibibytes — is
  refused rather than read. The bound is **private**: it is this door's
  own number, and a public bound is a promise anybody else may rely
  on.

### What the port found

- **A browser is reachable without a bundler.** The whole of the
  `wasm-bindgen` world is a convenience: a string is a pointer and a
  length, and a page can write bytes into a module's memory itself.
  Four exports and two buffers are the whole of it — and **none of it
  needs `unsafe`** beyond the attribute that gives the exports their
  names, because the module's memory is addressed by **numbers**, and a
  number is not an unsafe thing.
- **The registry survives the trip.** The ecosystem is built from the
  `inventory` the linker wrote, and `inventory` has a wasm path of its
  own. Measured here: the module holds **44 commands**, and they are
  the same commands the prompt offers.
- **A page cannot fetch a file off the disk.** The module and the
  page have to be **served**, so the bundle puts them in one directory
  and the person serves that. (The old tree's page has the same
  requirement and the same answer.)
- **The door's own words are the prompt's words.** The browser door
  has exactly one answer of its own — `quit`, which a page does not
  have — and everything else is the terminal door's.

### What it cost

- one application, one page, one check, and **seven tests**.

### What it does not settle

- **The page is text.** The kit's answers are text, and the canvas
  (§67) is a raster that the page does not draw.
- **The page is not a shell.** It has one input and one answer: no
  editing, no history, no completion — exactly as at the prompt (§75).
- **Nothing runs this in CI.** v2 had no CI job of its own, so the
  module was built by hand and the check was run by hand. That is now
  a workflow of its own (§83).

---

## 79. The host and the instance

The CLAP door could **read** a plugin: it found the entry, checked the
version, and read the descriptors (§76). But a plugin that is only read
is a file. The thing a program *uses* is an **instance**: made by the
factory, given a **host**, and lived through.

### What was built

- **The ABI crate gained the other half of the contract**: the
  **host** — what a plugin sees of the program that loaded it — and
  the **instance**, the plugin's own table of life.
- **The host is built and handed over**: its name, its vendor, its
  url, its version, and three functions the plugin calls when it wants
  something — a restart, a call back, a call on the main thread.
- **The factory makes plugins.** `create(id)` gives an instance, and
  the instance has a life: `init`, `activate` with the rate,
  `deactivate`, `start_processing`, `stop_processing`, `reset`, and
  `destroy`.
- **The reference plugin is a real plugin now.** It is made, it reads
  the host's name, it asks the host to restart it, it takes the rate
  it was activated at, and it frees itself when it is destroyed.

### What the port found

- **A host is a structure the plugin reads and calls back through.**
  The handshake is two words wide: the plugin reads the host's name
  and asks for a restart. The **request** is the only channel a test
  can see across the boundary — the plugin's own memory is the
  plugin's — and it is enough to prove that the structure crossed
  correctly.
- **The instance is not the file.** The file holds a factory; the
  factory makes instances; each instance has its own state and its own
  destruction. The instance's `destroy` frees the state the plugin
  made, so a host that never destroys **leaks** it.
- **A pointer to a thing that moves is a pointer to nothing.** The
  host and its state are boxed, because the plugin holds a pointer to
  each of them and may keep it.
- **The format's function pointers may be empty.** The reference
  plugin has no `process` — it does no audio — and the format says
  that is allowed. So the host must not call a pointer it has not
  checked, and every call in the host checks.

### What it cost

- one more structure in the ABI crate, one more part of the host, and
  **five tests**.

### What it does not settle

- **There is no audio.** The `process` call is not made: the block it
  takes is a structure the format defines and this arc does not
  model. That is the next arc, and it is the big one.
- **The host has no extensions.** The state, the log, the parameters
  and the ports are reached through `get_extension`, and this host
  answers with nothing: a plugin that wants a parameter or a port does
  not get one.
- **One instance at a time.** The host makes an instance and keeps it;
  there is no pool, no panel of plugins, and no state to save and
  restore.

---

## 80. The audio path

The instance had a life but no **work**: the `process` call — the one
every audio plugin exists for — was not made. This arc makes it.

### What was built

- **The ABI crate gained the block**: the audio buffer, the input and
  output event lists, and the process structure that holds them.
- **The host builds the block and hands it over.** The channels are
  **planar** — one vector a channel — because that is how the family's
  own audio is laid out, and the host writes the channel addresses into
  the buffer it gives the plugin.
- **The events are empty.** This host has no transport and no event
  list, and the format says a plugin may be given none.
- **The reference plugin does audio now.** It is a **gain**: it adds
  one to every sample that comes in and writes it out.

### What the port found

- **The audio crosses the boundary in both directions, and the test
  says so.** The plugin adds one; the test reads the output and finds
  one more than it put in. That single fact says that the samples
  arrived where the plugin expected them *and* left where the host
  expected them — which is the whole of what a host has to get right.
- **The channels are a table of addresses.** The format's buffer does
  not hold samples; it holds the **addresses** of the channels, and
  the channels hold the samples. So the host builds the table, and the
  vectors it points at must not move while the plugin is writing —
  which is why the block is borrowed for the length of the call.
- **A plugin that is not ready must not be asked to work.** The
  reference plugin says `ContinueIfNotProcessed` before it is
  initialised, and the host believes it. The status is the plugin's
  own word, and the host has nothing better to say.
- **The events are the part that is not built.** The format's events
  are a transport, a note, a parameter value — each one a structure
  with a size and a type, and a list that is read through two
  functions. This host gives the plugin an **empty** list, which the
  format allows, and the audio path works without them.

### What it cost

- three more structures in the ABI crate, one call in the host, and
  **two tests**.

### What it does not settle

- **There are no events.** No transport, no notes, no parameter
  values: the plugin is given none. A MIDI plugin needs them, and
  that is the next arc.
- **The ports are one port each way.** The host offers one input port
  and one output port, whatever the plugin says it has. A plugin with
  more ports needs the audio-ports extension, which this host does not
  have (§79).
- **The block is float.** The format's sixty-four bit half is not
  used, and neither is the latency or the constant mask.

---

## 81. The parameters a plugin holds

The host could make a plugin and could feed it audio (§80), and it
could not ask the plugin **anything**: not what its parameters are,
not what they are set to. An audio application that wants to show a
plugin's controls — or automate them — needs exactly that.

### What was built

- **The ABI crate gained the parameters extension**: the parameter's
  own information — its id, its name, its range, its default — and
  the plugin's table of them, and the host's half of the same
  extension.
- **The host asks the plugin for the extension** by its name
  (`clap.params`), and reads the whole table at once: how many
  parameters there are, what each of them is, and what each of them
  is set to.
- **The names are copied out of the plugin's own fixed-size arrays.**
  The format says a parameter's name is a short fixed thing, and the
  host that has to allocate for every parameter it reads is a host
  that allocates on the audio thread.
- **The reference plugin has parameters now**: a gain and a tone,
  with their ranges, their defaults and their values.

### What the port found

- **An extension is found by name, and it is the plugin's own table.**
  The plugin is asked for `clap.params` and answers with an address;
  the host then reads that table with the plugin's own functions. The
  shape is the same as the entry point's: a **data** answer, and
  functions behind it.
- **The names are arrays, not pointers.** Every other string in the
  format is a pointer to a C string, and this one is a **fixed-size
  array** with a zero byte at the end of the text. So reading a name
  is a copy of the array and a cut at the zero byte — and the host
  never has to free it, because the plugin never owned it separately.
- **Reading a value and setting one are different.** The plugin's
  table has `get_value` and **no** `set_value`: the format sets a
  parameter by **offering an event**, so setting one needs the event
  path that is not built yet. This arc reads; the next one writes.
- **The host's half of the extension is the plugin's way to ask.**
  A plugin that has changed its parameters asks the host to **rescan**
  them. This host has no such answer yet, and says so by having no
  extension of its own.

### What it cost

- three more structures in the ABI crate, one read in the host, and
  **one test**.

### What it does not settle

- **A parameter cannot be set yet.** The format sets one by offering
  an event, and the events are not built (§80). So the host can read
  a plugin's parameters and cannot move them.
- **The plugin's values are not flushed.** The extension's `flush` is
  where a plugin says "I have new values"; this host does not read it.
- **The host has no parameters extension of its own.** A plugin that
  asks the host to rescan its parameters gets nothing, and the format
  says that is allowed.

---

## 82. The event that moves a parameter

The host could read a plugin's parameters (§81) and could not move
one, because the format does not set a parameter by calling the
plugin: it **offers an event**, and the plugin reads it while it
works.

### What was built

- **The ABI crate gained the events**: the head every event has, and
  the parameter-value event the host offers.
- **The host's event list is a real list now.** The host keeps the
  events it has to give, and the plugin reads them through the
  format's own two functions: how many there are, and the one at that
  index.
- **A parameter is set by offering an event**, and the value reaches
  the plugin at the next block of audio — which is exactly what the
  format says.
- **The reference plugin reads the events** while it works, and takes
  the value the host offered.

### What the port found

- **The format's events are a list with two functions behind it**, and
  the host's own state is what the list points at. So the count of the
  events and the events themselves have to be **one thing with an
  address**: a list that pointed at a slice could not say how long
  the slice was.
- **An event is a header and a body.** Every event begins with the
  same head — the size, the time, the space, the type — so a list of
  events can be read without knowing what is in it. The host does not
  have to know the body to hand the event over, and it does not.
- **Setting a value is not calling the plugin.** The host offers, the
  plugin takes, and the value is there **afterwards**: the test sets
  the gain, processes a block, and reads the parameter back. That is
  the whole round trip, and it is the first thing in this shim that a
  person would call working.

### What it cost

- two structures in the ABI crate, a list in the host, and **one
  test**.

### What it does not settle

- **Only parameter values travel.** The format's events include the
  transport, the MIDI, the notes and the parameter changes; this host
  offers one kind and reads one kind.
- **The plugin's own events go nowhere.** The list the plugin pushes
  into is still empty, and the host does not read it: a plugin that
  wants to tell the host something cannot.
- **The events are not timed.** The head has a time, and the host
  writes zero into it.

---

## 83. The tree's own continuous integration

v2 is **excluded from the outer workspace**: it is a spike, it has its
own rules, and it has its own gates. So it had no CI at all — every
check in this README was run by hand, on one machine, by whoever was
working.

### What was built

- **A workflow of its own** (`.github/workflows/v2.yml`), running the
  v2 tree's house: the formatter, the linter, the tests, the browser
  door's module built for the browser, and the **outer** gate that
  reads v2's files.
- **The job is skipped when there is no v2.** On `main` — where the
  tree is the owner's own — there is no `v2/`, and the job says so
  instead of running on a tree that is not there.
- **The lock file is honoured.** Every cargo command runs `--locked`,
  and the warnings are errors, exactly as the outer tree's own
  workflow does it.

### What the port found

- **A tree that is not in the workspace cannot be in the workflow
  either.** GitHub reads the workflows from the repository's own
  root, so a workflow *inside* v2 would never run. The v2 CI is
  therefore a file in the outer tree that **skips itself** when the
  tree is not there — the same shape as the outer tree's own `wasm`
  phase, which skips with a reason when its tool is missing.
- **The outer gate reads v2, so the outer gate belongs in v2's CI.**
  The gate that checks the public bounds v2 names, and the deny
  attributes, and the layering, lives in the outer tree. So the v2
  workflow runs it — the owner's gate, run on the owner's machine,
  about the owner's tree.

### What it cost

- one workflow file, and no new code.

### What it does not settle

- **The workflow has never run.** It is written against the same
  actions the outer tree's own workflow uses, and it is **not** the
  outer tree's workflow: the outer tree's gates run on `main`, and
  this one runs wherever v2 is.
- **One machine.** The outer tree's workflow runs its gates on three
  systems; this one runs on one. v2 has no platform-specific code
  except the shared-library door, which is `unix`-only by name.
- **The mutation suite is not here.** The outer tree's instrument runs
  against the outer tree's gates, and v2's gates are not in it.

---

## 84. Where the music is

A plugin that plays has to know **when** it is playing: the position
of the music in beats and in seconds, the tempo, the time signature,
and whether it is playing at all. That is the **transport**, and the
format keeps it as an event — so the host offers it and the plugin
reads it while it works.

### What was built

- **The ABI crate gained the transport event**: the position in beats
  and in seconds (as the format's fixed-point numbers), the tempo, the
  time signature, and the flags that say which of them are there.
- **The host offers a transport** before a block of audio, and the
  plugin takes it while it works.
- **The events are a byte buffer now**, in both directions. The
  format's events are of different kinds and different sizes, and
  every one of them begins with a head that says how long it is — so
  the list is a buffer and a set of offsets, exactly as the format
  keeps them.
- **The plugin's own events reach the host.** The plugin offers an
  event and the host copies it out, and the host reads the parameter
  values among them.

### What the port found

- **The events are bytes, not structures.** The host's list was a list
  of parameter-value events, which is a list of **one kind** — and the
  format's list holds many kinds, each with its own size. Packing them
  into a buffer with the head at the front of each is the format's own
  shape, and it is the shape that lets the host hand over an event it
  does not understand.
- **A position in beats is a fixed-point number.** The format writes
  a beat as the position multiplied by `1 << 31`, so that a position
  keeps its fraction of a beat. The host's numbers are ordinary
  numbers, and the conversion is the host's business: the plugin
  sees the format's own.
- **The plugin's own events are how a plugin reports.** A plugin that
  wants to say "this parameter has moved" does not call the host; it
  **offers an event**, and the host reads it after the block. The
  reference plugin reports the tempo it was given, and the test sees
  the tempo twice: in the plugin's own parameter, and in the events it
  pushed.

### What it cost

- one structure in the ABI crate, two lists in the host, and **one
  test**.

### What it does not settle

- **The transport is whole-beats and whole-seconds.** The format's
  fixed-point positions are converted from the host's numbers, and the
  host's numbers are the ones the caller gave it — the kit's own time
  model is not in this path yet.
- **No MIDI.** The format's other events — the notes, the MIDI — are
  not built, so an instrument cannot be played.
- **The loop is not there.** The transport has four fields for the
  loop, and the host writes zeros into them.

---

## 85. A note reaches the plugin

The transport told the plugin where the music was (§84). A plugin that
is an **instrument** needs more than that: it needs to be **played**.

### What was built

- **The ABI crate gained the MIDI event**: the three bytes of a
  message and the port it is for.
- **The host sends a MIDI message** to a port, and the plugin reads it
  while it works — an event like any other.
- **The reference plugin counts the notes it was played**, and says so
  through a parameter, so the host can see that the message arrived
  whole.

### What the port found

- **A MIDI message is three bytes and an event is a header and a
  body.** The message is the body, the header says how long the whole
  thing is, and the plugin reads the list without knowing what is in
  it. So the host did not have to change at all to send a note: it
  wrote another event into the same buffer.
- **The test needs a way to see inside the plugin.** A plugin's memory
  is the plugin's, so the only things a host can see are what the
  plugin says and what it offers. The reference plugin counts its
  notes in a **parameter**, and that is a real thing a real plugin
  does — a parameter is how a plugin says something about itself.

### What it cost

- one structure in the ABI crate, one method in the host, and **one
  test**.

### What it does not settle

- **The message is three bytes.** The format's system-exclusive
  messages are longer and are a different event; this arc does not
  build them.
- **There is no note event.** The format has a richer note event, with
  a key and a velocity of its own, and this host sends the ordinary
  MIDI message instead.
- **Nothing plays.** A message reaches the plugin and the plugin
  counts it; there is no synthesiser here, and the audio the plugin
  makes is still the gain of §80.

---

## 86. The state a plugin carries

A plugin's **state** is the bytes that make it what it is: the
parameters it holds, the notes it is playing, the position of a
sampler. The host does not know what is in it — and that is the point:
**the state is opaque**, and a document can carry opaque bytes.

### What was built

- **The ABI crate gained the state extension** and the two **streams**
  it works through: one the plugin writes into and one it reads from.
- **The host saves the plugin's state** and gives it back. The bytes
  are copied out at once and the host does not look inside them.
- **The reference plugin's state is its parameters** — a count, the
  tempo, and each parameter's id and value — so the round trip is a
  real one: a parameter is moved, the state is saved, the parameter is
  moved again, and the state is given back.
- **The events a host offers are for one block.** They are cleared
  after the block, so a host that offered a note does not offer it
  again in the next block.

### What the port found

- **The state travels through streams, not through a buffer.** The
  format's save and load take a **stream**, and a stream is a pair of
  functions: write this many bytes, read this many bytes. So the host
  does not have to guess the size of the state in advance — the
  plugin writes as much as it has, and the host keeps it.
- **The events were being offered twice.** The host kept its events
  after the block, so a note offered once was offered again in every
  block after it — and a state given back was immediately overwritten
  by the old event. The rule is simple and the format says it: **the
  events are for this block**. The state round trip found it.
- **A plugin's state is the one thing a document can carry whole.** It
  is opaque by design: the host saves the bytes, the document holds
  the bytes, and the plugin is the only one who knows what they mean.

### What it cost

- three structures in the ABI crate, two streams in the host, and
  **one test**.

### What it does not settle

- **The state is not in a document yet.** The kit's document can carry
  opaque bytes (§12, the payload), and the host can save and restore
  them, but nothing joins them: there is no command that puts a
  plugin's state into a node, and no command that takes it out again.
- **The host's state extension answers nothing.** A plugin that says
  "my state has changed" is not heard; the host has no way to know
  that a saved state is out of date.
- **The state is bounded at sixty-four mebibytes.** A plugin that
  writes more than that is refused, and the bound is private to this
  host.

## 87. The plugin in the document

§86 ended with a gap: the host could save a plugin's state and the
document could carry opaque bytes, and **nothing joined them**. This
section joins them, and the join is a **battery** — the fourteenth one,
`craftkit-plugin-clap`, and the first one whose subject is a *program*.

### What was built

- **A node kind, `clap.plugin`.** Its payload holds the plugin's
  identity — the library it came from, its id, its name, its vendor and
  its version — the parameters the plugin says it holds, and **its
  state**, as base64.
- **Five commands.** `clap.list` reads the plugins in a library without
  touching the document. `clap.attach` opens the plugin and puts it in
  the document. `clap.detach` takes it out again. `clap.parameter.set`
  moves one parameter and keeps the state that follows. `clap.state`
  asks the plugin what it makes of the state the document holds.
- **The state travels as base64 in the payload** — the same answer the
  PDF battery gives for a stream, and the same answer the SDK's own
  `bytes` module gives.
- **No instance is ever kept.** A command opens the plugin, hands it the
  state, does its work and closes it. What the document keeps is what
  the plugin *said*.
- The **MCP server links the battery in**, so an agent can drive it: the
  server's tool list is **188 tools** with **all fifteen batteries**
  linked, measured the same way as every other number here.

### What the port found

- **A plugin is a program, and a document cannot hold a program.** So
  the document holds the *state* and the *name*, and the library is a
  file on disk like a picture: the document says where it is. The
  family's own mixer does the same thing — an insert names its plugin
  and carries its state, and the plugin itself is somewhere else.
- **The instance cannot be in the document, and it does not need to be.**
  A CLAP instance holds addresses into a library that is open, it cannot
  be sent between threads, and it is destroyed when the host is finished
  with it. A session is `Send` — the control channel holds one behind a
  mutex — so an instance in a session would be a `Send` problem before
  it was a document problem. The way out is the honest one: **the
  instance is an implementation detail of a command**, and what survives
  the command is the plugin's own bytes.
- **The state is opaque, and that is exactly what makes it carryable.**
  The host never looks inside it; the plugin writes it and the plugin
  reads it. So the document can hold it without pretending to
  understand it, and the plugin that gets it back finds its own memory
  exactly as it left it.
- **A parameter is set by offering an event, so the plugin has to work
  once.** The value does not arrive by being written into the plugin;
  it arrives in the block of audio. So `clap.parameter.set` runs **one
  frame of one channel** through the plugin, and the frame is not here
  to make a sound: it is here to deliver an event. The kit still has no
  audio thread, no sample rate of its own and no transport — this is a
  **seam**, and the block is the smallest thing that fills it.

### What it cost

- one crate, one node kind, five commands, one panel, **six tests**, and
  the state bound below.

### What it does not settle

- **The state is bounded at sixteen mebibytes** (`16 * 1024 * 1024`),
  and the bound is private to this battery. The base64 costs a third
  more than the bytes, and the host's own bound is sixty-four mebibytes
  (§86) — so the document's bound is the smaller one, and it is the one
  that decides what a document may carry.
- **The library path is in the payload and nothing checks it.** The
  document's own machinery for a file that is not there — the asset
  list, the stream list — does not see a plugin's library, so a
  document that names a plugin that is not installed is not refused at
  the door. It is refused when a command opens it, which is later and
  less clear.
- **Every command opens the plugin again.** A `dlopen` and a `dlclose`
  per command is cheap, and a plugin that takes a second to open pays
  that second every time. Keeping the instance alive would fix it, and
  it is exactly what the tree decided not to do.
- **The audio path is still not a document.** There is no sample rate,
  no block size and no transport in a document; the rate here is this
  program's, and a plugin's audio goes nowhere.

## 88. The ports

The host asked the plugin for everything else — its name, its
parameters, its state — and never asked it how many channels it has.
It guessed: one.

### What was built

- The ABI gained the **audio ports** extension: what one port is (its
  id, its name, its flags, its channel count, its type and the port it
  shares a buffer with) and the plugin's own table of them.
- The host **asks**: `audio_ports` and `channels`, and it remembers the
  answer when the plugin is activated — once, because a layout is not a
  thing to ask for on the audio thread.
- The host **builds the block**: `ClapInstance::block(frames)` makes a
  block in the plugin's own shape, so a caller does not have to know it.
- The **reference plugin declares** one stereo port in and one stereo
  port out, and **refuses a block that is not its shape**: it says it
  did not process the block, and it writes nothing.
- The **document says the shape**: a plugin node carries the channels
  in and the channels out, as the plugin said them.

### What the port found

- **A port is a group of channels, and the channel count is the
  plugin's own.** One stereo port is *two* channels: the number of
  ports and the number of channels are different numbers, and a host
  that reads the first as the second is a host that gives a plugin half
  its audio.
- **The plugin is the one who refuses.** The format says the plugin is
  activated with the layout it asked for, and a plugin that checks says
  it did not process the block. So the host's job is to **ask and to
  build**, and the plugin's job is to **refuse** — and a host that
  checks on the plugin's behalf is a host that has decided what the
  plugin wanted.
- **The extension is optional, and a host that cannot know says so.**
  The layout is `None` when the plugin does not have the extension: the
  host does not pretend to know, and a block cannot be built for it.
  That is a refusal in the right place — at the command, in words —
  rather than a wrong answer on the audio thread.
- **A Rust string is not a C string.** The port's type is a string the
  plugin owns, and the reference plugin's first answer was read *past
  its end*: the fixture's type was a `&str`, and a `&str` has no zero
  byte at the end of it. The format says C string, the fixture now ends
  its string with a zero byte, and the test says what the type is.

### What it cost

- one extension in the ABI, one method and one field in the host, one
  extension and one check in the fixture, one constructor in the block,
  **four tests**, and four lines in the battery.

### What it does not settle

- **The host still has no audio thread.** The layout is known and
  honoured, but nothing plays and nothing is recorded.
- **The in-place pair is read and not used.** A plugin can say that two
  ports share one buffer, and the host reads the number and does
  nothing with it.
- **The ports are not in the document as ports.** A plugin node carries
  two numbers — the channels in and out — and not the ports themselves:
  a port has a name and a type, and the kit has no place for those
  yet.
- **The host's own extensions are still nothing.** `get_extension`
  answers nothing at all: a plugin that asks the host for its
  parameters, for its state, or to rescan its ports is not heard. That
  is the next gap, and it is the other side of this one.

## 89. The word that the state has changed

§86 listed a gap: *"the host's state extension answers nothing. A
plugin that says 'my state has changed' is not heard."* This is the
other side of the state: the host's own extension, and the plugin's
word.

### What was built

- The host's `get_extension` answers **"clap.state"** — the **same id
  on both sides of the format** — with the host's own table, whose
  `mark_dirty` the plugin calls.
- The **reference plugin says so**: a value that changes is a state
  that changed, and the plugin asks the host to remember it.
- The word is good until the host **saves** the state or **gives it
  back**: after that, what the host holds is what the plugin holds,
  and the flag is cleared.
- The **document follows**: the battery says whether the plugin said it
  had changed, and the node's copy of the state is what the plugin
  saved.

### What the port found

- **The extension id is the same on both sides.** The plugin's table
  saves and loads; the host's table marks the state dirty. A host that
  knew only the plugin's side of the extension would hear nothing at
  all — which is exactly what happened.
- **"Dirty" is a word, not a state.** The format has no "clean" to say:
  the host clears its own flag when it saves the state or gives it
  back. So the rule is not "the plugin is clean" but **"what the host
  holds is what the plugin holds"** — and the flag is only the way the
  plugin says that is no longer true.
- **A host that does not hear it keeps a state that is out of date.**
  The family hit this too: a plugin that moved a parameter and a host
  that saved the state before the plugin said so had a document that
  lied about the plugin.

### What it cost

- one table and one function in the host, one call in the fixture, three
  lines in the battery, **four tests**.

### What it does not settle

- **The host's other extensions are still nothing.** The host answers
  "clap.state" and nothing else: a plugin that asks the host for its
  own parameters, or to rescan its ports, is not heard. The shapes are
  in the ABI and the tables are not in the host.
- **The word is nearly always "no" by the time the battery reads it.**
  The battery opens the plugin afresh, gives it the document's state,
  and reads the flag — and a plugin that has just been given its state
  has nothing to say. A live instance would keep the word between
  commands, and that is the instance the tree decided not to keep.
- **Nothing here runs on an audio thread.** The flag is an atomic, so
  it is safe to set from one, and the kit has no audio thread to prove
  it on.

## 90. The transport comes from the kit's time

The transport was the last thing the host offered **once**: the events
a host offers are for one block, and the transport was one of them. But
the transport says where the music *is*, and the music does not stop
between two blocks — so a host that offered it once told the plugin
where the music **was**.

### What was built

- The host **keeps** the transport and offers it with **every** block.
  The reference plugin counts the blocks that carried one and reports
  the count back, so a host can be *seen* to have offered it every
  time.
- The **plugin node carries a transport**: where the music is, in
  **ticks**; whether it is playing; and the **tempo** and **meter**
  maps.
- **Three commands**: `clap.transport.set` moves the music,
  `clap.transport.tempo` sets the tempo from a tick onwards, and
  `clap.transport.meter` sets the meter from a tick onwards.
- The transport the plugin is told about is **derived**: the beats, the
  seconds, the tempo and the meter at that tick, worked out with the
  kit's own time model (`craftkit-sound-time`, the library the sound
  battery uses).

### What the port found

- **The transport is not an event for one block.** Every other event
  the host offers happens once: a parameter value is set, a note is
  played. The transport is not that kind of thing — it is **where the
  music is**, and the answer changes with every block. So it is kept,
  and offered again and again.
- **The position is a tick, and the rest is derived.** The kit's unit
  is the tick — 960 to a quarter note — and the beats and the seconds
  are worked out from it. The format wants fixed-point numbers, the
  kit's model wants ticks, and the conversion is the host's business.
  Nothing is stored twice: the node holds the tick and the map, and
  everything else is worked out when it is needed.
- **The tempo is a map and not a number.** A piece changes tempo, and a
  single number cannot say *when*. So the document holds the events and
  the tempo at a tick is **derived** from them — and the test moves the
  music across the change to show the new tempo is the one in force.
- **A derived value that is stored is a value that goes stale.** The
  node says "tick 1920" and the map; the beats, the seconds and the
  tempo are worked out every time they are asked for. A document that
  stored the tempo as well would have two answers to the same question.

### What it cost

- one field in the host's instance, two payload structures and three
  commands in the battery, **four tests**.

### What it does not settle

- **The tempo map is in the plugin node's payload, and the sound
  battery keeps its own in its own nodes.** Two batteries, two places
  to say what the tempo is. The tree's answer — a battery is a document
  model of its own, and batteries do not depend on each other — is
  right, and it is also the reason the duplication is there.
- **The sample rate is the program's, not the document's.** The seconds
  are worked out at 48 kHz, because that is the rate the kit's own
  audio path uses and the rate the plugin is activated at.
- **Nothing moves by itself.** The transport is offered with every
  block, and the position only moves when a command moves it: there is
  no audio thread and no playhead.
- **The host still offers the plugin nothing but "clap.state".** The
  host's own parameters and the request to rescan the ports are still
  not answered.

## 91. The window

The kit had four doors — the terminal, the agent, the bridge and the
browser — and **no window**. A kit that a person cannot look at is a
kit that a person cannot use, and the README has said for a long time
that the front ends were not built.

### What was built

- **A window**: `apps/craftkit-window`, an `eframe` application over the
  same ecosystem the studio has. Three panes: the **commands** (every
  command in the registry, by area), the **document** (the tree of
  nodes), and the **plan** (what the document means).
- A **form** built from the command's own parameters: a number is a
  drag, a text is a line, a flag is a box, a node reference is a list
  of the document's nodes, and a kind is a list of the registry's
  kinds.
- **One way to run a command.** The window calls `Session::execute` and
  nothing else; the answer goes to the log.
- **A sixth rule in v2's own gate**: *a front end is thin*. It does not
  name `document_mut`, it does not name the evaluator's registry, and it
  does not hold the plan — with three tests that show the rule bites.

### What the port found

- **A front end that shows the plan must not keep it.** The plan is
  the evaluator's own value, and a window that holds one holds a value
  that goes stale the moment the document changes. So the window reads
  the plan when it draws and copies what it shows into rows of its own
  — and the gate refuses the word `Plan` in its source.
- **An optional parameter that nobody filled in is not sent.** The
  window's first version sent every field, and `node.add` refused:
  the *parent* was node zero, and zero is a node that is not there. The
  zero of a node reference is not "no parent" — it is a reference to
  nothing — so a field that was never filled in is **left out**, and
  the command's own default is the right answer.
- **The commands are the registry's, and the window has no list.** A
  command that is added to the registry appears in the window without
  the window being told, because the window asks the registry and
  draws what it says. That is the whole of "one way in", seen from the
  front.
- **A rule needs a name to bite on.** The gate reads the front end's
  **text**, because that is what a gate can read: the names are whole
  words, so `PlanRow` is not `Plan`, and a name inside a string is not
  a call. The gate is a function over a tree (§54), so a test can point
  it at a tree that is broken on purpose and say that the rule bites.

### What it cost

- one crate, one gate rule, **three gate tests** and **five tests of
  its own**, and one dependency the tree did not have: `eframe`.

### What it does not settle

- **There is no canvas.** The window draws a tree of nodes and a plan;
  the kit's document has no space to draw in, and a canvas would need
  its own camera controls (§ the outer tree's rule).
- **The window shows the plan as text.** The values are JSON, and a
  person would rather have a picture of them.
- **The window cannot open a file.** The commands are there —
  `doc.open` and `doc.open.bundle` are in the registry — but the form
  for a path is a line of text, and nothing checks it until the
  command does.
- **Nothing here is a drawing program.** The window is a way to *see*
  the kit and to run its commands; it is not the kit's canvas, and it
  does not pretend to be.

## 92. The window draws the document

§91 left one thing out: the window showed a **tree** of nodes and a
**plan**, and neither of them is a picture of a document. The kit has
had a canvas since the rasteriser was written (§67) — and nothing was
drawing into it.

### What was built

- The window **draws the document**, through the kinds: every node is
  offered to the kind that says what it is, and a node whose kind is
  not linked in is not drawn.
- The **camera is the session's**, so the picture is seen through the
  same controls every other front end uses — and the window draws it
  again only when the document, the camera or the command count has
  changed.
- The window links in **all fifteen batteries**, so the picture and
  the plan are of the whole kit and not of half of it.
- One more test: the picture is the size it was asked for, it is four
  bytes a pixel, and **the shape is in it**, in the colour it was
  given.

### What the port found

- **A shape in the colour of the background is a shape nobody sees.**
  The reference plugin paints its rectangle **white**, and the
  rasteriser's background is white: the first picture of a document
  was a blank page. The window was right and the picture was empty,
  and the difference was a colour in another crate. A test that only
  checked "the picture has the right size" would have passed.
- **Drawing is work, and a window that draws sixty times a second is a
  window that is always busy.** The picture is drawn again when the
  document, the camera or the command count has changed — a
  fingerprint, not a guess, and the count is the window's own.
- **A front end that draws must still not keep what it drew.** The
  picture is the window's own shape — pixels, and nothing else — and
  the window does not know what drew it.

### What it cost

- one method in the window, one dependency (`craftkit-raster`), **one
  test**, and the window's own picture of a document.

### What it does not settle

- **There is no interaction.** The picture is a picture: a person can
  look at it and cannot yet click it, and the hit-testing the core
  offers is not wired to the mouse.
- **The picture has no scroll and no zoom of its own.** It is drawn
  through the session's camera, and the camera is the session's state
  — which is right, and means the window has to offer the camera's
  controls before the picture can be moved.
- **The window still shows the tree and the plan as text.** They are
  the same information twice, and a person would rather have one.

## 93. The window you can use

§92 left the picture as a picture: a person could look at it and
could not touch it. This is the mouse.

### What was built

- The window **picks**: a point of the world is offered to every node's
  kind, and the **deepest** hit wins — a thing in a group is in front
  of the group that holds it.
- A click **selects**, through the registry: the window runs
  `select.node` like every other command, and the selection is the
  session's state.
- The **camera** moves and zooms: dragging pans it, the wheel zooms
  it, and the camera's own caps clamp the zoom.
- The rasteriser now uses the **camera's own mapping** instead of
  working it out again, so what is drawn and what is clicked are the
  same mapping.
- Four more tests: the pick, the selection, the camera and its caps.

### What the port found

- **There were two mappings, and they agreed by hand.** The camera
  already knew the way back from the screen to the world, and the
  rasteriser worked it out again from the camera's zoom and its
  offset. Two copies of one rule is two answers to every question, and
  they agree until a round number is in one of them. The rasteriser now
  asks the camera — and the tree's rule about **one set of camera
  controls** is the same rule, seen from the other side.
- **A click is a command.** The selection is the session's state, and
  the command that sets it is in the registry like everything else: the
  window has no way to select a node that the registry does not know.
- **The hit is the kind's own.** The window does not know what a shape
  is or where its edges are: it offers a point to the kind, and the
  kind says whether the point is inside it.

### What it cost

- two methods in the window, one line in the rasteriser, **four tests**.

### What it does not settle

- **Nothing can be dragged.** A click selects, and a shape does not
  move: the document's own commands for that are not there yet.
- **The wheel zooms about the middle and not about the pointer.** The
  camera's zoom is about its own middle, which is the simplest rule and
  not the one a person expects.
- **The tree and the plan are still text.** The picture is a picture
  and the rest is a list, and a person would rather have one.

## 94. The node moves

§93 left one thing: a click selects, and **nothing moves**. The reason
was not the mouse. It was that the document had no way to move a thing.

### What was built

- **A command, `node.move`**: a node with a place goes to another
  place, and it **keeps its size**. A node with no place is refused —
  a clip in a sound document is at a time and not at a place, and the
  command does not invent one.
- The window **drags**: a drag that starts on a node moves that node,
  and a drag that starts on the background moves the camera. What is
  under the pointer says which of the two it is.
- The node moves when the mouse is **let go**: the drag is many mouse
  events, and the command is one.
- One more test: the node moves, keeps its size, and the document says
  so.

### What the port found

- **A document model where a node cannot move is a drawing and not a
  document.** The batteries could *place* things — the vector battery's
  rectangle says where it is — but the core's own nodes had a place and
  nothing could change it. The gap was in the **commands**, not in the
  mouse: the mouse was ready and the registry was not.
- **A node with no place cannot be moved.** The rule is the sound
  battery's: a clip is at a time, and a time is not a place. A command
  that invented a place for it would be a command that invented a
  document.
- **The undo is the reason the move is one command.** The drag is many
  events and the command is one, so the journal has one entry and the
  undo has one step. A window that moved the node on every mouse event
  would have a journal of a thousand entries and an undo that took a
  thousand presses to get back.

### What it cost

- one command, two methods in the window, **one test**.

### What it does not settle

- **The shape does not follow the mouse.** It jumps when the button is
  let go, because a document that changed on every mouse event would
  have an undo nobody could use.
- **Nothing is resized.** A node's size is the size it was made with,
  and there is no command that changes it.
- **The wheel still zooms about the middle and not about the pointer.**

## 95. The appearance is the kit's own

The README has said for a long time that a document model is missing an
**appearance that is not a plugin's payload**. This is that, and the
thing it needed first: a kind that can **see** the document.

### What was built

- **The paint carries the document.** A kind's paint is given the
  document as well as the node, and that is the whole change: a kind
  that follows a reference — a path that takes its colour from a style,
  an entity that takes its colour from its layer — cannot follow it
  without the document.
- The **appearance** is the kit's own: an `Appearance` is a fill and a
  stroke, a node carries one, and a node may point at the node that
  holds one.
- The **kind** "core.appearance" and two commands in the SDK
  (`core.appearance.add`, `core.appearance.set`), and one in the studio
  (`node.appearance`) — a thing that uses an appearance points at it.
- The **vector battery** uses it: the appearance wins over the colours
  in the path's own payload.
- The **colour** is written one way: `Colour::from_hex` is the kit's,
  and the vector battery's own `#rrggbb` parser is gone.
- **Eight tests**: four in the SDK (the resolution, the precedence, the
  absence and the round trip) and four in the studio (the command, the
  refusal, the shared appearance and the colour in the picture).

### What the port found

- **A kind that cannot see the document cannot follow a reference** —
  and a document model in which every value is a copy is a document
  model in which twenty shapes are twenty colours to change, and one of
  them will be forgotten. The paint was the seam that had to change:
  not the payload, not the node, the **paint**.
- **The appearance does not replace the payload; it wins over it.** A
  path whose colours live in its own payload keeps them, and an
  appearance overrides them. Nothing is lost and nothing has to be
  migrated, and the new thing is there for the documents that want it.
- **A reference is followed once.** A chain of appearances is a document
  nobody can follow, and one hop is enough for a style that shapes use.
- **The colour is the kit's, not the plugin's.** The vector battery had
  its own `#rrggbb` parser, and every battery that took a colour would
  have had its own: the same spelling, written twice, is two things to
  get wrong.

### What it cost

- one field and one reference on the node, one type in the SDK, one
  module, three commands, twelve signatures in twelve batteries, one
  parser removed and **eight tests**.

### What it does not settle

- **The appearance is a fill and a stroke.** There is no opacity of its
  own, no dash pattern, no gradient, and no shadow: the kit says what
  an appearance is, and it says it in two colours and a width.
- **Only the vector battery uses it.** The other kinds keep their
  colours in their own payloads, and the next one to move is the CAD
  battery's layers.
- **An appearance is a plain node.** There is no stylesheet, no
  inheritance, and no "everything on this layer is this blue" — the
  reference is one thing pointing at another thing.
- **The window does not show the appearances.** It shows them in the
  **picture**, which is where they are, and a list of them would be a
  second thing to keep right.

## 96. The CAD battery draws

§95 left the CAD battery with the appearance and nothing to do with
it. This is the other half: the battery had a **model** — layers,
blocks, entities, extents — and **no picture at all**.

### What was built

- The CAD battery's entities **draw**: a line is a line, a circle is a
  circle, and a text is a text.
- The **colour is the layer's**: an entity is *on* a layer, the layer
  holds an appearance, and the entity is drawn in it. An entity with no
  layer is drawn in black.
- The **layer** takes a fill, written as `#rrggbb` — the kit's
  spelling, like every other colour in the tree.
- The **canvas's text takes a colour**. The seam said text is black,
  and a layer's text is not black: the text now takes a colour like
  everything else that is drawn.
- Two more tests: the line is in its layer's colour, and the entity
  with no layer is in black.

### What the port found

- **The model was a table and a flat store, and not a picture.** The
  CAD battery had layers, blocks, entities and extents, and every one
  of them said "not drawn". A port of the *model* without the *picture*
  is a document model with no way to see it — and the port of the
  **vector** battery, which came first, had drawn its paths from the
  start. The gap was that a battery can be ported twice: once for what
  it *holds* and once for what it *looks like*.
- **The colour was the layer's all along, and the layer could not say
  what it was.** The layer's payload held a `color` — an index or a
  name — and **nothing resolved it**: it was a field with no meaning.
  Now the layer holds an appearance, and the entity that is on the
  layer is drawn in it.
- **The seam said text is black.** The canvas drew text in a colour of
  its own, because the seam had no colour to give it. That is what a
  seam is for: it is the place where the tree's own rules are written
  down, and a rule that is wrong in the seam is wrong everywhere.

### What it cost

- one seam changed (the text takes a colour), one macro body, three
  drawings, one parameter on a command, **two tests**.

### What it does not settle

- **The circle is a polygon of sixty-four segments.** The kit has no
  curves in its drawing except the ones a path carries, and a circle
  is drawn as a polygon of straight lines.
- **The canvas's text has no letters.** The canvas draws the box each
  character would occupy, and the picture says *there is text here* and
  not what it says. — **Answered in §540**: the canvas draws the kit's
  own letters.
- **Nothing can be picked.** The entity's hit test still says no, so a
  line cannot be clicked in the window.
- **The layer's old `color` field is still there.** An index or a name
  in the payload, and nothing resolves it: the appearance is how a
  layer says what colour it is, and the old field has not been taken
  away.

## 97. The line can be picked

§96 left one thing: the CAD entities are drawn and cannot be **hit**.
The window picks with the hit test, so a line that cannot be hit cannot
be picked, and a drawing nobody can touch is a picture.

### What was built

- The CAD entities are **hit**: a line is hit by a point near it, and a
  circle is hit by a point near its **edge**.
- The **tolerance** is the battery's own — one unit of the world's — and
  not the core's: the core has no unit and no opinion about how near is
  near.
- Two more tests: the line is hit on it and missed away from it, and
  the circle is hit on its edge and **not** in its middle.

### What the port found

- **A line has no inside.** The vector battery asks "is the point inside
  the path", and a CAD line has no area at all: the question is "is the
  point **near** it", and the answer is a distance and not a
  containment. The two batteries share a seam and not a question.
- **The middle of a circle is the inside of it, and the inside is not
  the circle.** A hit test that asks the wrong question here picks the
  wrong thing: a click in the middle of a hole would select the hole.
- **The tolerance belongs to the battery.** One world unit is fine for
  a drawing in millimetres and would be wrong for a drawing in pixels,
  and the kit does not know which one it is looking at.

### What it cost

- two questions answered in one macro body, one distance, one constant,
  **two tests**.

### What it does not settle

- **The text is not hit.** The box a text occupies is not a thing the
  battery knows: the canvas draws the box of each character from its own
  metrics, and the battery cannot ask. The kit's typeface (§128) makes
  the box knowable in principle — a battery could ask the typeface how
  wide a character is — and this battery does not.
- **The tolerance does not follow the zoom.** A line is a pixel wide at
  a high zoom and is hard to hit, and a line is thin at a low zoom and
  is easy to hit by accident. The hit test has no camera.
- **The insert is not drawn and not hit.** It is a reference to another
  block, and the picture says nothing about it.

---

## 98. The undo is a command

§91 gave the window a way in and nothing to undo with. The history was
already there — it is a tree of states (§33) — but nothing could reach
it, because a front end that called the session's own history would be
a front end with a second way into the design.

### What was built

- Two commands in the registry: `doc.undo` and `doc.redo`. They are
  **mutating** commands and not **undoable** ones: they move along the
  tree of states and they add nothing to it.
- The window grows two buttons that run the two commands by name.
- The front-end rule grows two more forbidden words: `undo` and `redo`
  beside the document and the evaluator.

### What the port found

- **A rule that reads the text reads the comments.** The first version
  of the front-end rule searched the source for the word `undo` and
  found it in a sentence that explained what the rule was. A rule that
  a comment can break is a rule about prose, and the tree has one of
  those already and does not need another.
- **So the rule reads the code.** A stripper blanks comments and string
  literals before the rule looks, and the raw strings are handled
  separately because their delimiters are not the same. A front end may
  now say "undo" in a label and be told nothing.
- **The redo takes the newest branch.** The history is a tree, and a
  change made after an undo starts a new branch. Going back into an
  older branch needs a branch number, and a number is not a parameter
  the kit's own types carry: the redo takes the newest and says so.

### What it cost

- two commands, one stripper, two buttons, **three tests** (two for
  the rule and its stripper, one for the history itself).

### What it does not settle

- **The redo does not choose a branch.** The tree holds more than one
  way forward and the command takes the newest.
- **The buttons are not a menu.** The window shows the commands a
  panel's form would show; the undo and the redo are on the window
  itself, and the menu that would hold them beside everything else is
  not built.

---

## 99. The wheel zooms about the pointer

The camera had a zoom and the window had a wheel, and the wheel zoomed
about the **middle** of the screen. That is how a camera behaves when
nobody has thought about it, and it is wrong in the way that only a
person at the keyboard can feel: the world slides out from under the
hand while the hand is busy turning the wheel.

### What was built

- The camera gains a zoom **about a point of the screen**. The point is
  in screen units measured from the middle — the same units
  `to_screen` gives and `to_world` takes — and the camera works out
  for itself which point of the world is under the pointer and puts it
  back under the pointer.
- The window's wheel uses it, and the window gains a `zoom_at` of its
  own so that a front end with a pointer can say where it is without
  working out the world's mapping a second time.

### What the port found

- **A zoom that is clamped still holds the pointer.** The wheel hits
  the cap at the top and at the bottom, and the naive version moves
  the anchor when it clamps: the zoom stops and the world jumps. So
  the anchor is put back **after** the clamp, and a wheel that hits
  the cap changes the zoom as far as it goes and not one step further.
- **The camera already knew the answer.** The mapping from the screen
  to the world was `to_world`, and the whole of the new method is
  that one call and one subtraction: the new rule was not new
  geometry, it was the old geometry used in a second place.

### What it cost

- one method on the camera, one on the window, **four tests** (three
  on the camera and one on the window).

### What it does not settle

- **The handle is not scaled.** A resize handle drawn at every zoom
  would need the camera's zoom in the drawing, and the picture is
  drawn by the kinds and not by the window.
- **The wheel does not pinch.** A touch screen has two fingers, and
  the camera has one factor: a pinch is two wheels at once, and the
  kit has no gesture of its own.

---

## 100. The node can be resized

A move was a command and a resize was not: the window could drag a
node across the world and could not make it bigger, and a drawing
program that cannot resize is a drawing program that cannot draw.

### What was built

- A command `node.resize`, beside `node.move`: a resize changes the
  size and keeps the place, and a move changes the place and keeps the
  size. Two commands and not one, because a command that did both
  would be two commands wearing one name and its undo would take back
  more than the person asked for.
- The **handle** is the kit's own: `Rect::resize_handle` says where it
  is and `Rect::resized_to` says what a drag of it makes. The anchor
  stays where it is, and the rectangle grows and shrinks from its
  corner.
- The window grows a third kind of drag: a drag on a **handle**
  resizes the node, a drag on the node moves it, and a drag on the
  background moves the camera. The handle is a screen size, so how
  near it is in the world's units comes from the camera's zoom.

### What the port found

- **The handle has to be in the kit and not in the front end.** The
  first version put the handle in the window, and the window is a
  front end: two front ends would have put it in different corners,
  and a handle that moved would resize the other way round. The
  rectangle in the kit now says where the handle is.
- **A size is not a direction.** A drag past the left edge would make
  a rectangle with a negative width, and a rectangle with a negative
  width cannot be drawn and cannot be picked. So the kit gives an
  **empty** rectangle and not a negative one, and the command refuses
  a size of nothing.
- **A node with no place has no size either.** The command refuses it
  for the same reason the move does: a note in a document is at a time
  and not at a place, and the kit does not invent one.

### What it cost

- two methods on the rectangle, one command, three kinds of drag,
  **eight tests** (three on the rectangle, three on the command and
  two on the window).

### What it does not settle

- **The handle is not drawn.** The picture is drawn by the kinds, and
  a kind does not know it has a handle: the window can drag a handle
  it cannot see.
- **Only one handle.** A rectangle has four corners and the kit knows
  one of them: a resize that keeps the anchor keeps one corner still,
  and the other three would each need their own rule.

---

## 101. The node can be hidden

Every drawing program has the idea: a layer that is turned off, a guide
that is in the way, a shape that is not part of the picture yet. v2 had
no way to say it. The only way to have a thing that was in the document
and not on the screen was to take it **out** of the document — and
taking it out loses it.

### What was built

- A **flag** on a node: `hidden`. A hidden node is not drawn and not
  picked, and it is still in the document.
- The **ninth version** of the format, which is the version that writes
  the flag. Every earlier version writes it away, so a file of an
  earlier version says nothing about hiding.
- Two commands: `node.hide` and `node.show`. Two commands and not one
  with a boolean: a boolean says "yes" or "no" instead of saying what
  to do.
- The kit owns what hidden means: `Document::walk_visible` skips a
  hidden node and **does not go into it**, so a hidden group hides what
  is in it. The window draws and picks with that walk and decides
  nothing.

### What the port found

- **A flag on the core is a version of the format.** The node is
  written by a function every version shares, so adding a field to it
  changes every file the kit writes — and a file of the eighth version
  must not carry a field the eighth version cannot explain. So the
  field is the **ninth** version's, the earlier writers take it off,
  and the step from the eighth to the ninth says what walking forward
  means.
- **The version is still derived from the document.** Nothing was
  declared and nothing was configured: the document is written at each
  version in turn and read back, and the first version whose round trip
  gives the same document back is the one. A document with a hidden
  node comes back smaller from the eighth version — the flag is
  dropped — so it is written at the ninth. **The rule found the new
  version by itself.**
- **"Hidden" has to be one thing and not two.** The first version of
  the window skipped hidden nodes when it drew and did not skip them
  when it picked, and a node that could not be seen could still be
  picked. So the kit has the walk, and both the drawing and the
  picking use it.

### What it cost

- one flag, one version of the format, one step, two commands, one
  walk, **eleven tests** (six in the studio — four about the file and
  two about the commands — and one in the window, plus the four the
  new version needed elsewhere).

### What it does not settle

- **A hidden node is still in the plan.** The plan is what the document
  means, and a hidden node means something; the plan says what, and
  nothing says that it is not to be seen.
- **Nothing is hidden by default.** A document that a program opens has
  everything shown, and a program that wants a layer turned off has to
  turn it off.

---

## 102. The hit test sees the document

§97 left the insert undrawn, and drawing it found the seam that was
holding it back.

### What was built

- The **hit test** is given the **document**, like the paint has been
  since §95. Every kind in the tree was changed with it.
- The **insert** draws the block it inserts: the block's own entities,
  moved by the insertion point.
- The insert is **hit as the block it inserts**, for the same reason it
  is drawn as the block.

### What the port found

- **A kind that cannot see the document cannot follow a reference.**
  That is what §95 said about the paint, and it was true of the hit as
  well: the paint seam had been changed for the same reason, and the
  hit seam was left behind. The insert was the thing that found it,
  because an insert is a reference and a reference is followed by
  looking at the document.
- **A drawing and a picking that disagree are worse than either of
  them alone.** The insert was drawn as a block and hit as a point: the
  picture showed a line and the click found nothing on it. Whichever
  way the seam went, the two had to agree.
- **A reference is followed once.** An insert of an insert draws the
  block and stops: the second insert is not drawn, and the drawing
  does not go on forever. The rule of §89 — a reference is followed
  once — held for the picture as it does for the appearance.

### What it cost

- one parameter on every kind's hit test (**twenty-seven files**), the
  insert's paint and hit, a move that moves every point of a geometry,
  **two tests**.

### What it does not settle

- **The block's own entities are still drawn where they are.** A block
  in a drawing is drawn as well as inserted, so a line in a block is
  on the screen twice: once where the block is and once where the
  insert puts it. Real drawings keep the definition out of the way.
- **The insert is drawn in its own colour.** The block's entities keep
  their own layers, and an insert of a block of red lines is drawn in
  the colour of the insert's layer and not in the colours of the
  lines.

---

## 103. The layer is the kit's own

The CAD battery had layers: a layer was a node, an entity was *on* a
layer by an edge, and the rule that said "an entity is drawn in its
layer's colour" lived in that battery. Every other battery that wanted
a layer would have wanted the same rule and written it again, and two
rules that say the same thing drift apart.

### What was built

- The **edge** that says a node is on a layer is the kit's own:
  `core.on_layer`. The kit says what follows from it:
  - a node with **no appearance of its own** takes the **layer's**;
  - a node on a **hidden** layer is **hidden**;
  - the layer is followed **once**.
- The **appearance** rule is now one rule: own appearance, then the
  one it points at, then the layer's.
- Three commands: `core.layer.add`, `core.layer.on` and
  `core.layer.off`, and a kind of the kit's own, `core.layer`.
- The **CAD battery** uses the kit's edge and the kit's rule: its own
  `colour_of` is gone, and the entity is drawn in the colour the
  appearance rule gives it.

### What the port found

- **The layer's kind is not the kit's business.** The first version of
  this made the CAD battery's layers into `core.layer` nodes, and
  that was two kinds of layer and one more place for a rule to live.
  Now the kit cares about the **edge**: a CAD layer is a CAD table
  row, an artist's layer is an artist's node, and the kit's own
  `core.layer` is a plain node with a name. Every kind of layer works
  with the kit's rule.
- **A node is on one layer.** The edge says which, and a second edge
  would be two answers to one question — so the second one is refused,
  and taking a node off takes it off every layer.
- **The appearance rule was already the layer's rule.** §95 said the
  appearance wins over the payload, and the layer was the next step
  that nobody had taken: the fallback chain was own, then the one it
  points at, and then nothing. Now it is own, then the one it points
  at, then the **layer's**.

### What it cost

- one module in the kit, three commands, one kind, two rules in the
  document, and the CAD battery's own rule taken away,
  **five tests**.

### What it does not settle

- **A layer has no order.** Layers are in the order the document holds
  them and that is all: a layer that is above another one, a layer
  that is locked, a layer that is printed — none of that is here.
- **A node is on one layer and not many.** A drawing that puts a
  shape on two layers has to say which one it is on, and the kit says
  there is one.

---

## 104. The kit draws the document

The window had a resize handle it could drag and not see: the picture
was drawn by the window, node by node, and the handle was not a node,
so it was not drawn.

### What was built

- The **session paints**: every node that is not hidden is offered to
  the kind that says what it is, and the **resize handle** of the
  selected node is drawn on top.
- The window and the studio ask the session to draw, and keep no rule
  of their own about what is seen.
- The handle is a **screen** size: eight pixels across, whatever the
  zoom, and how big that is in the world's units comes from the camera.

### What the port found

- **A front end that draws the document has a second rule for what is
  seen.** The window walked the document and asked each kind to paint
  it, and the studio did the same, and the two were the same rule
  written twice — which is how the CAD's colour rule drifted from the
  appearance rule until §103 joined them. The rule is now in the kit,
  and the front ends ask for a picture.
- **The handle belongs to the kit too.** The handle is the corner
  opposite the anchor, and §100 put that in the kit's rectangle; the
  place it is **drawn** is the same question. A handle drawn by a front
  end is a handle that two front ends draw in two places.

### What it cost

- one method on the session, one constant, the window and the studio
  asking for a picture, **five tests**.

### What it does not settle

- **The handle is black.** The kit has no chrome, no theme and no
  colour that means "this is a control": the handle is drawn in the
  drawing's own colour, and a drawing on a black background has a
  black handle.
- **Only the selected node has a handle.** A drawing program shows the
  handles of everything that is selected, and the selection here is a
  list of nodes, so every selected node has one.

---

## 105. The order is the tree's order

A drawing program has an order: what is drawn later is on top. v2 had
the drawing and no way to change the order.

### What was built

- The document can **bring a node to the front** and **send it to the
  back**.
- Two commands for it: `node.front` and `node.back`.

### What the port found

- **The order is not a number a node carries.** The first thought was a
  `z` on the node: a number that says where it is in the drawing. But
  a number is a number that two nodes cannot share, and a group has to
  renumber everything under it when anything moves. The **tree already
  has the order** — where a node is among its brothers — and the
  picture and the picking walk it in the same order, so bringing a
  node to the front is moving it to the end of the list it is in.
- **The tree's order was already the drawing's order.** Nothing had to
  change in the raster or the picker: the last node of the walk is the
  one on top, and the picker takes the last hit. That was true before
  this arc and was not written down anywhere; now it is.

### What it cost

- two methods on the document, two commands, **three tests**.

### What it does not settle

- **A group is one thing in the order.** A group is drawn where it is
  in the tree, and everything in it is drawn with it: there is no way
  to say that one thing in a group is above a thing outside it.
- **There is no "bring the selection to the front".** The commands
  take one node, and the selection is the session's state and not a
  parameter.

---

## 106. The snapshot frames the document

`doc.snapshot` wrote a PNG — and the picture was empty when the
document was not at the origin.

### What was built

- The snapshot is seen through a camera **worked out from the bounds
  of the document** and the size that was asked for. The session's own
  camera is not moved: a picture made for somebody else must not move
  what somebody else is looking through.
- The snapshot is drawn by the **kit**, so a node that is hidden is not
  in it.
- The kit says which of its two drawings is the **document** and which
  is the **view**: `paint` draws the document and `paint_view` draws
  the document and the chrome. The handle is chrome, so a picture of
  the document has no handle in it and the picture on the screen has
  one.
- The document grows a **bounds**: the box that holds everything that
  is somewhere. A node with no place is in none of it — a clip in a
  sound document is at a time and not at a place.

### What the port found

- **A picture of the origin is not a picture of the document.** The
  snapshot was seen through the session's camera, which is right for
  the screen and wrong for a file: a person who has panned away and
  asks for a snapshot gets a picture of where they were looking.
- **The chrome is not the document.** The first version of this drew
  the snapshot with the same method the window draws with, and the
  snapshot had the handle in it — a little square that means "this can
  be dragged" in a picture that is going into a file. So the kit has
  two drawings now: the document, and the view of it.

### What it cost

- one method on the session, one on the document, one on the camera,
  the snapshot changed in two lines, **four tests**.

### What it does not settle

- **The bounds are the bounds of the places.** A node that is drawn
  outside its own rectangle — a stroke that goes past the edge, a
  picture that is drawn larger than its node — is not in the bounds,
  and a snapshot may cut it off.
- **The snapshot has no background.** The picture is white where
  nothing is drawn, and there is no colour for the page.

---

## 107. The selection moves as one

A person who picks twenty nodes and moves them expects **one** undo,
and not twenty.

### What was built

- A command `select.move`: it takes **no node**, because the
  selection is the session's state, and it moves every node in the
  selection that has a place.
- The window follows: a drag on a node that is selected moves the
  whole selection, because that is what a person means when they drag
  one of the things they picked.

### What the port found

- **A command that takes a list is a command that takes a list.** The
  first thought was `node.move` with a list of nodes in it, and the
  second was a loop of `node.move` in the front end. The loop is
  twenty undo steps and the front end holding a rule; the list is one
  command and one step, and the thing it moves is the selection —
  which the session already has.
- **A node with no place is left alone, and the command says how many
  were left.** A clip in a sound document is at a time and not at a
  place: the command does not invent one, and it says how many of the
  selected nodes had none, so that a person is not left wondering why
  one of them did not move.

### What it cost

- one command, two methods on the window, **five tests**.

### What it does not settle

- **Only the move.** A selection that is resized, deleted or brought
  to the front is still one node at a time.
- **A node that is in a group is moved on its own.** Moving a group
  moves what is in it, because the group is what is in the tree; but
  moving a node that is inside a group does not move the group.

---

## 108. The document can be emptied

The kit could add nodes, move them, hide them and resize them, and it
could not take one out. A document that cannot be emptied is a
document that grows for ever.

### What was built

- A command `node.delete`: the node goes out of the document and its
  **edges** go with it.
- A command `select.delete`: the whole selection, in **one** undo
  step.
- A command `relation.add`: the kit could not make an edge either,
  and an edge that no command can make is an edge that only a battery
  can make — which is the thing the batteries were for before the kit
  had one.
- The document's removal takes the edges of the **whole subtree**: a
  node that is deleted takes what is in it, and their edges go with
  them.

### What the port found

- **An edge to nothing is an edge that says something false.** The
  document could lose a node and keep the edges that pointed at it: a
  clip that plays a source that is not in the document any more. The
  removal was in the core and the edges were in the core, and they
  were not together.
- **The kit could not make an edge.** Every battery that needed one
  wrote its own command — `cad.entity.layer`, the sound battery's
  edges — and a program that wanted an edge of its own had no way to
  make one. So the core has the command now, and the batteries' edges
  are the same edges.
- **The undo brought the edge back**, because the history keeps the
  document as it was and an edge is part of the document. That was
  already true; it had just never been asked.

### What it cost

- one method on the document changed, three commands, one module in the
  kit, **six tests**.

### What it does not settle

- **The edges of a node that is deleted by a battery are that
  battery's business.** The core's removal takes the edges; a battery
  that deletes a node by another way must take them itself.
- **There is no undelete.** The undo is the undelete, and a document
  that was saved and closed is gone.

---

## 109. The panels are shown

A **panel** is a battery's own window of commands: it names the commands
it shows. The batteries declared them, the ecosystem registered them,
and **no front end showed them**.

### What was built

- The window shows the **panels** first, and the commands that no panel
  shows after them, so that every command is shown **once**.
- The window keeps the panels of the ecosystem it was built with: a
  panel is a **contribution** and not a session's state — the session
  is what a person changes, and a panel is what a battery shows.

### What the port found

- **A command that is shown twice is a command with two doors.** The
  first version of the window showed the panels *and* every command by
  area, so the panel's commands were in both. The commands that no
  panel shows are the rest, and the window asks for those.
- **The panels are not the session's state.** The session holds what a
  person changes — the document, the history, the selection — and a
  panel is what a battery brings. So the window keeps them beside the
  session and not inside it.

### What it cost

- two methods on the window, one field, the window's own list of
  commands, **one test**.

### What it does not settle

- **The panel is a list of commands and not a view.** It says what it
  shows and nothing about how it shows it: a panel with a form in it,
  or with a picture, is not a thing the kit has.
- **A panel whose commands are not linked in is shown with a hole in
  it.** The commands that are not there are left out, and the panel
  does not say how many it wanted.

---

## 110. The control channel drives the window

The control channel could only serve a session of its own. So a
program that had a session already had **two** documents, and one of
them was a copy.

### What was built

- The channel can serve a session the caller **keeps**: the same lock
  on the same session, so the two are one document.
- The **window** opens a channel over its own session when it is asked
  to: `--control PORT`, with the token in `CRAFTKIT_CONTROL_TOKEN`.
- The window's session is behind a **lock**, and the lock is taken
  anyway when it is poisoned: a panic in one thread must not take the
  window down with it.

### What the port found

- **A copy of a document is a document that drifts.** The channel's
  session and the window's session would each have their own history,
  their own selection and their own camera, and the person at the
  keyboard would be looking at one of them while the agent was driving
  the other.
- **The token is asked for and not invented.** The window takes the
  token from the environment and opens nothing without it: a channel
  with a token a program can guess is a channel with no token, and the
  program that opens it is the one that decides who may drive it.

### What it cost

- one method on the channel's server, the window's session behind a
  lock, the window's methods adapted, **one test** — and the test is
  the point: it runs a command over the channel and looks at the
  window's document.

### What it does not settle

- **The channel cannot drag.** The old tree's channel had `ui.click`,
  `ui.screenshot` and `ui.render`. v2's channel has `ui.render` (§111)
  and `ui.click` (§112) — the agent can see the document and click in
  it — but not a drag, and not the family's `ui.screenshot`: this
  tree has no screen to take one of, only a document to draw.
- **The window redraws when it is told to.** The picture is drawn again
  when the document, the camera or the command count has changed, and
  a command that arrived over the channel changes the document — so
  the window redraws — but nothing says so out loud.

---

## 111. The channel can show the window

The channel could change the document and could not see the result. An
agent that cannot see the result is an agent that has to believe it.

### What was built

- The window gives the channel a way to ask for the **picture**, and
  the channel has a method that asks: `ui.render`.
- The picture travels as **base64 in one line**, and a line is
  bounded — so the picture is three hundred and twenty by two hundred
  and forty, which fits in the line and is big enough to see what
  changed.
- A program with **no window** says so: the answer names the reason,
  and it is not an empty picture.

### What the port found

- **The channel knows the session; the window knows the screen.** The
  two are different things and the seam between them is a trait the
  window implements: the channel asks for a picture and hands the
  answer over, and it never asks the session to draw one.
- **A picture that does not fit in the line is a picture that cannot
  be sent.** The first version drew at eight hundred by six hundred,
  and the answer was refused by the reader before the client saw it —
  the line is the bound and the picture has to live inside it.
- **The token was printed.** The options had a derived `Debug`, and it
  printed the token: a token in a log is a token in the world. So the
  options say "a token" and not what it is.

### What it cost

- a trait in the channel, a method on the wire, a picture in the
  window, the options' `Debug` written by hand, **two tests**.

### What it does not settle

- **The picture is the document and not the window.** The panels, the
  form and the log are chrome, and they are not in the picture: an
  agent sees what the document looks like and not what the program
  looks like.
- **The picture is drawn at a fixed size.** The window draws at the
  size of its panel, and the channel's picture is three hundred and
  twenty by two hundred and forty whatever the window is.

---

## 112. The channel can click in the document

The channel could show the window but not touch it. So an agent could
look at the document and could not do anything to it except run a
command with a name it already knew.

### What was built

- A method `ui.click`: the point is in the **world's** units, and the
  node under it is picked through the kinds and selected through the
  registry.
- The window's control handle now does two things: it shows the
  picture and it clicks.

### What the port found

- **A click over the channel is the same click a person makes.** The
  first thought was a click by the *name* of a button, and the
  buttons of a window are the commands — so a click by name would have
  been `engine.execute` with extra steps. A click at a **point** is
  different: it goes through the kinds' hit test and the selection,
  which is the thing a person does and the thing an agent wants.
- **The point is in the world's units.** A click at a point of the
  screen is a click at a point of the world that the camera has
  already found, and the camera is the session's — so the click and
  the drawing agree.

### What it cost

- a method on the wire, a method on the window's control handle,
  **one test**.

### What it does not settle

- **The channel has no pointer of its own.** It can click (§112) and
  drag (§113), but a drag is a drag from one point to another: a
  person can hold the mouse down and see the node follow it, and the
  channel cannot.
- **The click does not move anything.** It selects, and a selection is
  the session's state; the agent moves a node with a command, and the
  command is the same one a person gets.

---

## 113. The pointer is the window's own

The drag lived in the **toolkit's** layer of the window: the code that
decided what a drag meant was next to the code that knew about
pixels. So the channel could click in the document and could not drag
anything in it.

### What was built

- The **pointer** is the front end's own: three functions — down,
  dragged, up — that take points in the **world's** units.
- The toolkit's layer only says where the pointer is on the screen;
  the camera turns that into a point of the world, and the pointer
  says what that point means.
- The window has the pointer's state, and the channel's control handle
  has one too, so the channel can drag what it clicks.

### What the port found

- **The handle's tolerance was twice as big as the handle.** The
  handle is drawn eight pixels across, and the tolerance was the whole
  of it, so the pointer was "on the handle" when it was up to eight
  pixels away — and on a document whose rectangles are ten units
  across, that is most of the node. The tolerance is **half** of the
  handle now: the handle is a square centred on the corner, and the
  pointer is on it when it is within half of it.
- **The drag moves the selection when the node is selected.** That was
  a rule in the toolkit's layer, where it belonged to nobody; it is in
  the pointer now, and it is the same rule for the window and for the
  channel.

### What it cost

- one module in the window, the toolkit's layer reduced to forwarding
  the pointer's events, a method on the wire, **five tests**.

### What it does not settle

- **The drag is a drag from one point to another.** A person holds
  the mouse down and watches the node follow it; the channel sends
  the two ends and does not see the middle.
- **The pointer is the front end's, and not the kit's.** A program
  with no screen has no pointer, and the kit has no rules about what
  a hand does.

---

## 114. The selection is drawn

The window drew the **handle** of the selected node and not the
selection itself, so a person could not see what they had picked.

### What was built

- The kit draws the **frame** of the selection, around every selected
  node that has a place, and the handle beside it.
- The frame is **chrome**, like the handle: it is drawn by the view
  and not by the document, so a picture of the document — a snapshot
  for a file — has no frame in it.
- The line is a **screen** size, like the handle: one pixel across,
  whatever the zoom.

### What it cost

- one frame in the kit's chrome, **one test**.

### What it does not settle

- **The chrome is black.** The kit has no theme, and a selection on a
  black drawing is a selection nobody can see.
- **Only the nodes are framed.** A node with **no place** is selected
  and is not framed: there is no box to frame it in.

---

## 115. A locked node cannot be changed

The document had a way to say "do not show this" and no way to say
"do not touch this".

### What was built

- A **locked** flag on a node: it cannot be moved, resized or deleted.
- Locking a **layer** locks everything on it, because the layer is the
  thing that is locked.
- Two commands: `node.lock` and `node.unlock`.
- The flag is the **tenth** version of the format, and the version was
  derived by the rule itself: a document with a locked node comes back
  smaller from the ninth version, so it is written at the tenth.
- One locked node refuses the whole of a **selection** move: a move
  that left the locked ones behind would be a move the person did not
  ask for.

### What the port found

- **Locked is not hidden.** The first thought was one flag with two
  meanings, and the two meanings are not the same thing: a hidden
  node cannot be **seen** and a locked node cannot be **touched**, and
  a node can be one without being the other. So they are two flags.
- **The version tag sorts as a string.** The tenth version is
  `craftkit/10`, and the registry sorts its ids as strings: so
  `craftkit/10` comes *after* `craftkit/1` and *before* `craftkit/2`.
  The bridge's list of formats says so, and a version number that
  reaches two digits does not sort where a person expects it.

### What it cost

- one flag, one version of the format, one step, two commands, the
  commands that change a node now check the lock,
  **five tests**.

### What it does not settle

- **A locked node can still be selected.** The lock says the node
  cannot be changed, and not that it cannot be picked: a person can
  look at a locked thing and see what it is.
- **The lock does not reach the payload.** A node with a payload of
  its own — a grid cell, a clip — may have a command of its own that
  changes it, and that command has to ask the lock itself.

---

## 116. The VST3 ABI, written down as shapes

The second plugin format. The CLAP battery had its ABI written down as
a crate of data (§79), and VST3's is the same idea with a different
contract.

### What was built

- A crate `craftkit-vst3-abi` with **no `unsafe` in it at all**: the
  shapes a host reads out of a plugin, the tables it calls through,
  and the entry it looks for.
- The shapes carry the SDK's layout: the factory's information has its
  three **fixed-size** strings, the interface tables are the SDK's
  thirteen, eleven and twelve pointers in the SDK's order, and the
  tests say what the sizes are.
- The crate is in the layering table at **L0**, beside the CLAP ABI:
  a binary contract two sides share is data, and data belongs at the
  bottom.

### What the port found

- **The entry is a function and not a data symbol.** CLAP exports the
  address of a struct; VST3 exports `GetPluginFactory`, and calling it
  gives back the factory. That is a different door, and the host has to
  know which door it is opening.
- **There is no version number in the ABI.** CLAP has one, and the
  host checks it. VST3 has none: the host asks for the interface it
  wants by **FUID** and the plugin either gives it or does not. What
  there is, is the **SDK** the plugin was built against, and the crate
  says which one it describes.
- **The crate does not carry the published FUID bytes.** They live in
  the SDK's published headers, and this tree does not copy files it
  was not written for: a program that talks to a real plugin reads
  them from the SDK it was built against.

### What it cost

- one crate of shapes and **five tests** about the layout.

### What it does not settle

- **There is no host yet.** The crate is the contract and not the two
  sides of it: the host that opens the library and the fixture that
  exports the shapes are the next arcs.
- **The FUIDs are not here.** A program that talks to another program
  of this tree uses the ones its own plugin says, and a program that
  talks to a real plugin brings its own.

## 117. The VST3 stream and the tree's own interface ids

§116 wrote the VST3 ABI down as shapes, and said plainly that the
contract was there and **the two sides of it** were not. This section
fills in the three shapes that neither side could do without: the
stream a state travels through, the instance the factory hands back,
and the names the two sides ask each other by.

### What was built

- **The stream** (`IBStream` and `Stream`): a state is a set of bytes,
  and the format does not say "give me the bytes" — it says "read from
  this stream" and "write to this one". The stream is the **host's**
  own, and its **first word is the address of its table**, so the
  plugin can find the functions and the host can keep the bytes behind
  them.
- **The instance** (`Instance`): the factory hands back an **object**,
  and the object's first word is the address of its table. That is how
  a virtual table is laid out, and it is what the format means when it
  says that an object implements an interface.
- **The tree's own interface ids** (`COMPONENT_FUID`,
  `AUDIO_PROCESSOR_FUID`, `EDIT_CONTROLLER_FUID`): the published bytes
  belong to the SDK's headers, which this tree does not copy. So the
  tree names its **own** interfaces, and the host and the fixture agree
  on them because both of them are written here.
- **The reference count**: every interface counts references the same
  way, and the pair comes **first** in the table, because the format
  puts it there.
- **The audio buffers**: the **wide** shape, with the addresses of the
  channels in it. The SDK has a narrow field for the same thing beside
  it, and this tree holds the one a program can actually use.

### What the port found

- **A binary contract has parts that are not the contract.** The
  published FUID bytes are numbers the format's own headers hold, and
  copying them would have put a file this tree was not written for
  inside it. The **shape** of the contract — sixteen bytes that stand
  for an interface, asked for by value — is what a tree can write down
  without copying anything, and it is enough for two programs of this
  tree to talk.
- **An object implements an interface by its first word.** The
  factory's pointer is the factory's table, because the factory has
  one interface and no state of its own; an instance's pointer is the
  **object**, and the object carries its table. Two rules, and a host
  that mixed them up would read the wrong function.
- **A state is a stream and not a buffer.** The format says "read from
  this stream", so the shape is a **table of four functions** and not
  a byte array, and the host is the one that owns the bytes.

### What it cost

- four shapes and **three tests** about the layout.

### What it does not settle

- **One object cannot hold two tables.** The component and the audio
  processor are one object in the format, and this shape cannot say
  so: the table is the first word, and the first word is one thing.
  §120 says what the tree does about it.
- **The FUIDs are not the SDK's.** A program that talks to a real
  plugin asks for the SDK's bytes instead, and the shape of the call
  is the same either way.

## 118. The reference VST3 plugin

§117 wrote down the shapes; this section is the **other side** of them:
a plugin that exports them, built by the compiler and opened from the
disk.

### What was built

- **A `cdylib`** — `craftkit-vst3-fixture` — that exports one
  **function**, `GetPluginFactory`. The host finds it by name, calls
  it, and gets the factory back.
- **One class**, `Craftkit::Fixture`, with **one parameter** (a gain),
  **one bus in** and **one bus out**, and a state of four bytes: the
  gain itself, in the plugin's own words.
- **The plugin does the one thing a plugin is for**: what comes in
  comes out, louder or quieter.
- **The unsafe is in one module**, and every block in it says why it
  is sound: it is writing through a pointer the host gave it.

### What the port found

- **The entry is a function, and the symbol is found by name.** Craftkit's
  own door finds a function and calls it; CLAP's entry is a *variable*
  whose value is a pointer; VST3's is a function again. Three doors,
  two shapes.
- **A name in a pattern is a binding and not a constant.** The
  fixture's first draft asked which interface it was with a `match`
  over the interface id, and the arm bound the id instead of matching
  it — so *every* request took the first arm, and the plugin made a
  component when it was asked for a processor. The ids are now
  **compared**, and the lesson is the tree's own: a constant in a
  pattern is a thing that can be *read*, not a thing that can be
  *bound*.
- **The plugin's own state is the plugin's own.** The fixture saves
  four bytes and reads four bytes back, and the host never looks
  inside them: it cannot, and it does not try.

### What it cost

- one crate, **four tests**, and the **seventh** crate on the list of
  exceptions.

### What it does not settle

- **The audio processor is a second instance.** In the format it is
  the *same* object as the component, seen through another table, and
  §117's shape cannot say that. The fixture gives it a state of its
  own, and says so here rather than hiding it.
- **It is a fixture**: one class, one parameter, one bus each way.

## 119. The VST3 host

The last piece of the format: the door that opens the library, calls
the entry, and drives what comes back.

### What was built

- **A crate `craftkit-vst3`** that opens a shared library with
  `dlopen`, finds `GetPluginFactory` with `dlsym`, **calls** it, asks
  the factory what it holds, makes an instance, and drives it.
- **The table is resolved once.** The factory's pointer is the
  factory's table; an instance's pointer is the object, and the host
  reads the object's first word **when the instance is made** and never
  again. The host also knows **which** of the two tables it holds, so
  a host that asked for the processor cannot go on to call the
  component's methods.
- **The state travels as bytes.** The host cannot read a state — the
  bytes are the plugin's own — so it asks for them, carries them
  whole, and gives them back when the plugin is opened again.
- **The stream is the host's**: four functions over the host's own
  bytes, in a table the plugin finds in the stream's first word.
- **Twelve tests** against the real library: the entry, the class, the
  buses, the parameter, the state, the audio, and the two refusals.

### What the port found

- **A door that is called and a door that is read are different
  doors.** The CLAP host reads a pointer out of a symbol's address;
  this one calls the symbol. The loader is the same, and everything
  after it is not.
- **The host is written in the safe language, and the format's tables
  are safe too.** The tables hold *safe* function pointers, so calling
  one is not an unsafe operation: what is unsafe is **reading** the
  table out of the plugin's memory, and that is the only place the
  crate says so.
- **The instance is never kept.** It holds addresses into a library
  that is open, and it is destroyed when the host is finished with it.
  A caller opens the plugin, uses the instance and lets it go; what
  travels is the state.

### What it cost

- one crate, **twelve tests**, and the **sixth** crate on the list of
  exceptions.

### What it does not settle

- **The processor is not the component**, for the reason §118 gives.
- **The published FUIDs are not here**, so this host talks to the
  plugins of this tree and not to a plugin built against the SDK.

## 120. The VST3 plugin in the document

§87 put a **CLAP** plugin in the document. This section puts a **VST3**
one there, and the differences between the two are the point.

### What was built

- **A node kind, `vst3.plugin`.** Its payload holds the plugin's
  identity — the library it came from, its class, its name, its vendor
  and its version — the parameters the plugin says it holds, and **its
  state**, as base64.
- **Six commands.** `vst3.list` reads the classes in a library
  without touching the document. `vst3.attach` opens the plugin and
  puts it in the document. `vst3.detach` takes it out again.
  `vst3.parameter.set` moves one parameter and keeps the state that
  follows. `vst3.state` asks the plugin what it makes of the state
  the document holds. `vst3.process` runs a block of audio through the
  plugin and says what came out.
- **The parameter is named.** A CLAP parameter is asked for by a
  number and a VST3 parameter is asked for by a **name**, so the
  command takes a name and the document keeps it, because the name is
  what the plugin says.
- **The value is the one a person sees.** The format's own value is
  **normalized** — between zero and one — and the plugin says the
  range it takes in its own units. So the document carries **both**,
  and the command takes the plain one.
- **The audio goes through.** `vst3.process` is a measurement and not
  a stream: the kit has no audio thread, and the block is here to show
  that the plugin's own path works.
- The **MCP server links the battery in**, so an agent can drive it:
  the server's tool list is **188 tools** with **all fifteen
  batteries** linked, measured the same way as every other number
  here.

### What the port found

- **A parameter is named, not numbered.** Two plugin formats, two
  ways of naming the same thing, and the battery that speaks each one
  says it in its own terms. The kit does not flatten them: the
  document keeps what the plugin said.
- **A normalized value and a plain value are two values.** The
  format's value is a position between zero and one, and the plugin's
  range is in millimetres or decibels or whatever it is. The document
  carries both, and the command takes the plain one because that is
  what a person has.
- **A value outside the range is held at the end of it.** The plugin
  would clip it anyway, and a document that says what was *asked for*
  is not saying what the plugin *did*.

### What it cost

- one crate and **seven tests**.

### What it does not settle

- **The state is bounded at sixteen mebibytes** (`16 * 1024 * 1024`),
  and **a block at sixty-four thousand samples** (`64 * 1024`). Both
  are this program's, and both are private to the battery.
- **There is no audio thread**, so `vst3.process` measures and does
  not stream: the kit's transport is in ticks (§90), and a stream of
  samples would be a second clock.

## 121. The AU ABI, written down as shapes

The third plugin format, and the first whose door is the **operating
system's**.

### What was built

- A crate `craftkit-au-abi` with **no `unsafe` in it at all**: the
  four-character codes, the properties and their scopes, the shape of
  the audio, the buffer list, and the unit's own **function
  signatures**.
- The codes are written out as the characters they are: a code is a
  name, and a number is not readable.
- A unit is asked about itself with **three numbers** — what is
  wanted, where, and which one — and the **scope** is the interesting
  one: the same property id means different things on the input, on
  the output and on the unit as a whole.
- The audio is a **buffer list**: a count and the buffers, each with
  the address of its samples. The format's list is variable in length,
  and this shape holds **two**, because the kit drives one bus in and
  one bus out. A bound is not negotiable and a shape is (§9): the tree
  says what it can carry and refuses the rest.
- The crate is in the layering table at **L0**, beside the other two
  ABIs.

### What the port found

- **A format's door can be the operating system's.** CLAP's door is
  the plugin's own entry; VST3's is a function the plugin exports; AU's
  is the **component manager**, which finds a unit and makes an
  instance of it, and which is the platform's. A tree that is not on
  that platform has no component manager, and the shape cannot hold
  one.
- **The strings are the platform's too.** Several of AU's properties
  answer with a **CoreFoundation string** — the name of a unit, the
  name of a parameter — and CoreFoundation is macOS's. So the names are
  not in this shape at all, and a program that wants them reads them
  where the platform has them.
- **A code is four characters and not a number.** Every OSType in the
  format is a four-character code, and the numbers are only how the
  codes travel.

### What it cost

- one crate of shapes and **five tests** about the layout.

### What it does not settle

- **There is no component manager here**, and there is not going to be
  one: it is the platform's. §123 says what the tree does instead.
- **There is no battery**, and the reason is the format's own: AU's
  parameters and its state are CoreFoundation's, so a document has
  nothing portable to carry yet.

## 122. The reference AU, exported by name

The other side of the contract: a unit that exports it, built by the
compiler and opened from the disk.

### What was built

- **A `cdylib`** — `craftkit-au-fixture` — that exports the **six
  functions** the format names: the two that read and write a
  property, the two that set the unit up and take it down, the one that
  puts it back as it was, and the one that renders a block.
- **One symbol of this tree's own**: `craftkit_au_instance`, which
  hands back an instance. The name is in this tree's namespace so that
  it cannot be mistaken for the format's.
- The unit is an **output unit** that passes the audio through, and it
  knows its rate, its shape and how many frames it takes in one slice.
- **The unsafe is in one module**, and every block in it says why it
  is sound: it is writing through a pointer the host gave it.

### What the port found

- **A unit is six functions and a bundle.** The format's unit has no
  struct to read and no table to walk: the host finds each function by
  name and calls it. That is a different shape from CLAP's table and
  from VST3's factory, and it is the same kind of door as craftkit's
  own (§34).
- **The instance is not the unit.** A bundle holds the **code**, and an
  instance holds the **state**, and the state is the platform's
  manager's to make. This tree makes it with its own door, and says
  so in the name of the door.
- **The audio's direction is a convention.** The format's render call
  is given the **output** buffers, and the input arrives through a
  callback the host sets — and a callback is the platform's style of
  saying "here is the input". This tree has no such thing: the first
  buffer is what went in and the second is what comes out. That is the
  tree's convention and it is written down in both places.

### What it cost

- one crate, **two tests**, and the **tenth** crate on the list of
  exceptions.

### What it does not settle

- **The unit has no parameters and no state.** A parameter's name is a
  CoreFoundation string, and the format's own state is the platform's
  to hold. So there is nothing here for a document to carry — which is
  why there is no battery yet.

## 123. The AU host, opening a unit and driving it

The last piece of the format: the door that opens the bundle and drives
what comes back.

### What was built

- **A crate `craftkit-au`** that opens a bundle with `dlopen`, finds
  the six symbols with `dlsym`, makes an instance through the tree's
  own door, and drives it.
- **Every symbol is found at once**, when the bundle is opened: a
  bundle that is missing one is refused at the door and not halfway
  through a render.
- **The properties are copied out at once**, because they are the
  unit's own memory and they may change the next moment. The rate, the
  shape of the audio and the size of a slice are the three the tree
  asks for.
- **Nine tests** against the real bundle: the symbols, the instance,
  the rate, the shape, the slice, the audio, and the two refusals.

### What the port found

- **A host that finds its symbols one at a time finds them too late.**
  The bundle is opened once and every symbol is looked up then, so a
  bundle that is not a unit is refused before anything is called.
- **The instance borrows the bundle.** An instance holds addresses into
  a library that is open, so the host's instance **borrows** the plugin
  rather than holding a copy of it: the compiler says so, and a
  program that dropped the bundle early would not compile.
- **The tree's own door is named in the tree's own namespace.** A
  symbol called `craftkit_au_instance` cannot be mistaken for one of
  the format's, and the host says in its own words that it is using the
  tree's door and not the platform's.

### What it cost

- one crate, **nine tests**, and the **ninth** crate on the list of
  exceptions.

### What it does not settle

- **There is no battery**, and there will not be one until the format
  has something portable to carry: its parameters and its state are
  CoreFoundation's.
- **The component manager is still not here.** The tree's own door
  makes an instance; the platform's would make one from a unit it
  found, and that is the piece this tree does not have.

## 124. A kind may say where its node is

A node's place is the place the **document** gives it, and that is the
right answer most of the time. The arc before this one found the case
where it is not: a clip in a sound document is at a **time**, and a
time is not a place.

### What was built

- **The kind spec grows a `place`**: `Option<fn(&Node, &Document) ->
  Option<Rect>>`, and `None` by default. A kind that says nothing has
  the place the document gives it.
- **The derive writes it** from a flag on the attribute —
  `#[node_kind(..., place)]` — the same way the evaluation is written
  from its own flag. A kind with the flag must have the method, and the
  compiler says so.
- **`Session::place_of`** asks the kind and falls back to the document.
- **`Session::bounds`** asks **every** kind: the bounds that frame a
  document whose geometry is derived and not in the file.
- **The chrome asks** — the frame of the selected node — so that a node
  whose place is derived can be seen to be selected.
- **The snapshot frames the derived bounds**, so a document whose
  geometry is not in the file is still pictured properly.

### What the port found

- **A derived value in the document is a value that goes stale.** The
  obvious way to give a clip a rectangle is to write it into the
  document when the clip is added. And then a track added above it
  moves every clip below it, and the document has to be rewritten to
  match — a value that is worked out twice, once by the person and once
  by the program, and the two will not agree.
- **The core never matches on a kind, and this seam keeps it that
  way.** The core asks the kind where its node is, and the kind
  answers. It is the same shape as the paint seam and the hit seam: the
  core does not know what a clip is, and it does not need to.
- **The selection needed the seam too.** The chrome reads the node's
  rectangle. Without the seam, a node whose place is derived could be
  picked but not *seen* to be picked — the frame would be drawn around
  nothing.

### What it cost

- one field, one flag, two methods, and the studio's snapshot asking
  the session instead of the document.

### What it does not settle

- **A kind that says where its node is says it every time.** The place
  is worked out when it is asked for and not when it is stored, so a
  kind that works is asked more than once. §125 says what that costs.
- **A kind that says nothing still has the document's answer**, and the
  two are not compared: a kind that derives a place and a document that
  holds one are two different places, and the kind's wins.

## 125. The timeline: the sound document gets a place

§19 found that a sound document has no place in it, and left it there
with the reason written down. This arc is the answer, and it is not the
obvious one.

### What was built

- **The timeline**: across is **time** — one world unit is one second,
  and the document's own sample rate says how many samples that is —
  and down is the **row**, which is a track's place among the tracks.
- **The place is derived.** A clip's rectangle is worked out from its
  span and from the row of the track it is under, and it is **not** in
  the document.
- **A clip is a bar**, a track is the line along the top of its row, a
  marker is a line down the timeline, and a clip can be **picked**.
- **The snapshot frames the timeline**, because the bounds ask the
  kinds.

### What the port found

- **The order of the tracks is the order of the tree**, and there is no
  other order to be had: the row of a track is its place among the
  tracks, the tracks are the document's roots, and the order of the
  tree is the order of the drawing (§105).
- **A playlist is a row with a different set of clips on it**, so it
  has the row of its track and draws nothing: the row is already drawn.
- **The mixer and the timeline are two different pictures.** The
  timeline shows the tracks and their clips; the mixer shows the
  strips and the buses and the routing between them. A document can be
  both, and the kit does not confuse them.

### What it cost

- one module, **three tests** in the battery and **three** in the
  studio.

### What it does not settle

- **The derived row costs a walk.** Finding the row of a track means
  walking the document, so drawing a document of `n` sound nodes costs
  `n` walks — a few tens of thousands of comparisons for a document of
  a few hundred nodes. A document big enough for that to hurt would
  want the rows **stored**, and that is a different design.
- **The tempo and meter events are not drawn.** Their time is in
  **ticks** and the world is in **seconds**, so placing them would need
  the tempo map, and that is the next arc. — **Built in §126**: the ruler
  is the row above the tracks, and the tick is worked out against the
  tempo map.
- **The mixer is still not drawn.** Its model is written (§20.4) and its
  picture is not. — **Built in §127**: the buses are rows under the
  tracks, the sends are the lines between them, the inserts are the boxes
  at the left of a row and the automation lanes are the curves on it.

---

## 126. The ruler: the tempo and the meter are marks above the tracks

§125 left one thing on the timeline unplaced, and it was not an
oversight: the **tempo** and **meter** events carry their time in
**ticks**, and the world's time is in **seconds**.

### What was built

- **The ruler**: a row of its own, **above** the tracks, at minus one.
  It is where the things that are not on a track live.
- **A tick is converted, not drawn.** The event's tick is worked out
  against the **tempo map** — the map built from the document's own tempo
  events — and the mark is placed at the resulting second.
- **A tempo event and a meter event are both marks**: a short vertical
  line in the ruler's row, and both can be picked.

### What the port found

- **A tick is not a length.** A tick is a **musical position**: how many
  subdivisions from the start. It becomes a length only when the tempo
  says how fast the music is, so the ruler cannot be drawn from the
  event's own number — it needs the map, and the map is a **derived
  value**, derived from the very events being drawn.
- **The map is not stored.** It is rebuilt from the document every time
  the ruler is placed, and that is the same rule as §528: a value that
  is derived does not live in the document, because a value in the
  document goes stale.
- **The tempo changes the past.** A tempo event at tick 960 changes
  where the mark at tick 1920 lands, and it changes nothing before it.
  So the ruler is not a table of positions — it is the map, drawn.

### What it cost

- one place function and two paint functions, and **one test** in the
  battery for the tempo's mark.

### What it does not settle

- **The map is rebuilt per event.** Placing `n` tempo events builds the
  map `n` times, so the ruler costs a walk per mark. A document with
  many tempo events would want the map built **once** per frame, and
  that is a change to the seam, not to the kinds.
- **The ruler has no scale.** It has marks and no numbers, because a
  number it cannot draw is not worth computing. The kit **has a
  typeface** now (§128), so the bar numbers of a real ruler are within
  reach — and they are not drawn yet. — **Built in §131**: the ruler
  counts by bars or by seconds, and the numbers are the kit's own
  letters.

---

## 127. The mixer is a picture

The mixer's **model** was written long ago (§20.4): a bus is a node, a
send is a node with a relation, an insert is a node with a plugin's
name, and an automation lane is a node with a parameter's name. What
was missing was the **picture** — and a model nobody can see is half a
model.

### What was built

- **The buses are rows under the tracks.** The mixer is under the
  timeline: the tracks are the rows above, the buses are the rows
  below, and a bus is drawn as the **line along the top** of its row.
- **The sends are the lines.** A send is the **edge** of the mixer's
  graph, and an edge is drawn where it goes: a line from the track's
  row down to the bus's row.
- **The inserts are the boxes at the left of the row.** An insert is not
  at a time, so it is at the **start** of the track — the way a mixer
  shows the slots of a strip.
- **An automation lane is a curve on the row**, drawn to **fill its
  row**: the kit does not know what range a parameter takes, so what is
  seen is the **shape** of the automation and not the absolute values.

### What the port found

- **The mixer is not a canvas of its own.** It is the same world as the
  timeline — seconds across, rows down — and it is simply **below** the
  tracks. Two pictures, one coordinate system, one kit.
- **A send needs both of its ends.** The send node says *what it is* and
  the relation says *where it goes* (§21), so the line's length is
  worked out from the bus's row and the track's row: the edge knows its
  own place because both of its ends have one.
- **A lane's value has no range.** The lane's points are the parameter's
  own values, and the kit does not know the parameter, so the curve is
  **scaled to the row**. This is a decision, not a shortage: what the
  lane shows is the shape of the automation, and a mixer that showed
  absolute values would need to know every parameter's range — which is
  the audio engine's business, and v2 has no audio engine yet.

### What it cost

- four tests in the battery — the bus's row, the send's line, the
  insert's box and the lane's curve — and **one** in the studio.

### What it does not settle

- **The mixer's rows are counted by walking.** The bus's row is the
  number of tracks plus its index among the buses, so drawing a mixer
  of `n` buses costs `n` walks, the same cost as §125's rows.
- **Two sends to the same bus at the same place are one line.** The
  sends are spaced along the row by a fixed amount so that two lines
  are not drawn on top of each other, but a mixer that draws one line
  per send would need a way to say which is which.
- **There is no audio.** The mixer is a picture of a mixer: the routing
  is in the plan (§20.4) and the sound is not made. That is the audio
  engine, and it is not built. — **Answered in §135**: the sound is
  made, offline — the document is rendered into a file.

---

## 128. The kit's own typeface, drawn in code

The kit had **no font**, and that was not an oversight and not a
principle: it was a **missing thing**. The rule of the tree is that
nothing on screen is an asset — no artwork, no font file, nothing that
is not drawn in code — and the icons have followed that rule since the
first picture. The letters had simply never been asked for.

### What was built

- **`craftkit-font`**: the kit's own typeface, a table of **glyphs**,
  each a set of **strokes** on a shared grid.
- **The grid**: the baseline at zero, the **cap height** at two, the
  **x-height** at one and a half, the **descender** at minus a half.
  Every letter is drawn on it, so the letters line up with each other
  and a caller does not have to know where a letter starts.
- **The em is the cap height**: a caller that says ten gets capitals
  ten tall, and one grid unit is therefore half the em.
- **The letters**: the capitals and the small letters, the ten figures,
  and the marks a sentence needs — a full stop, a comma, a colon, a
  semicolon, a dash, a bracket, a quote and the rest.
- **The typeface is a library and not a plugin**: a typeface is data —
  strokes and numbers — and it knows nothing about a document, a canvas
  or the kit. The caller brings the size and the place, and the library
  brings the outlines.

### What the port found

- **The no-asset rule was never about the letters.** It was about not
  shipping a file, and a letter is a few numbers in a source file —
  exactly like an icon. The rule that kept the artwork out kept the
  typeface out too, and nobody had noticed that a typeface drawn in code
  is not artwork at all.
- **A typeface is not a metric.** The port's own text battery had said
  for a long time that "the kit has no font, so the widths come from
  the caller" (§58) — and that was **right**, and is still right. What a
  line breaker needs is a width, and a width is a number in a document.
  What a **drawing** needs is an outline, and an outline is a shape.
  The two are separable, and the separation is what let the typeface
  arrive without touching the model, the breaker or the format.
- **The letters are a monoline.** A capital is a skeleton of straight
  strokes and a small letter is the same skeleton at two thirds of the
  height. That is what a plotter font is and what a monoline engraving
  is, and it is what a tree that draws its own icons is already good at.
- **A typeface has no name and no file.** The kit's letters are called
  what the tree calls them, and there is nothing to install.

### What it cost

- one crate, one module of strokes and one of tests: **ten tests**, and
  they are the rules a table has to keep — every letter and figure is
  there, every glyph is wider than nothing, no two characters share an
  entry, the letters are proportional, a string is as wide as the sum
  of its letters, an unknown letter is **left out** rather than drawn
  as a box, and no letter is taller than the em or hangs below the
  descender.

### What it does not settle

- **The typeface does not kern and does not shape.** A pair of letters
  is not adjusted against each other, and there is no ligature and no
  substitution. A caller that wants kerning applies it to the advance,
  which is what a metric is for.
- **The canvas still has no font of its own.** The kit's typeface is a
  library that a *battery* draws with, and the canvas's `text` is still
  a box (§67). The rasteriser could draw the kit's typeface for it, and
  does not: that is a change to a seam and not to a kind.
- **The letters are plain.** There is no bold, no italic and no
  hinting, and a document that wanted them would want a **family** of
  typefaces rather than one.

---

## 129. The paragraph is drawn, and a letter can be hit

§58's battery had a paragraph that was broken into lines and was **not
drawn**: the letters had no outlines. Now they have some.

### What was built

- **The paragraph is drawn** with the kit's own typeface, and the
  drawing is the letters' **strokes** — paths, one per stroke.
- **The metric says how wide a letter is, and the typeface says what it
  looks like.** The glyph is stretched by whatever the metric says
  against what the typeface would have had: a metric that agrees with
  the typeface stretches nothing, and a monospaced one stretches every
  letter to the same width, which is what monospaced text looks like.
- **The metric gained a size** — the **em**, in millimetres, the height
  of the capitals — and a metric that does not say has the default size.
- **A letter is a line and not a box, and the hit says so.** A pointer
  is on a letter when it is within half a stroke's width of one of the
  letter's lines: the middle of an `o` is a hole, and a pointer in the
  middle of an `o` is on nothing.
- **The paragraph has a place**: the block of its lines, as wide as the
  measure and as tall as the lines. So the snapshot frames a paragraph
  and the selection chrome can point at it.

### What the port found

- **A hit test has to ask the right question.** The first version hit
  the **box** of a letter, and the box of an `o` has a hole in it — so
  a pointer in the middle of a letter was not on the letter. The
  question is not "is the point inside" but "is the point **on** the
  line", and the answer is a **distance** — the same question the CAD
  battery asks of a line (§97), and the same answer.
- **The drawing and the measuring are two different things.** The
  breaker measures a word with the metric's **runs** and **pairs**, so a
  metric with a kerning pair measures the word a little narrower than
  the letters are drawn. The drawing places one glyph per character.
  The divergence is written down rather than hidden.
- **A letter's cell is wider than its ink.** The cell is the advance —
  the space from one pen to the next — and the ink is usually narrower,
  because that is what the space at the side of a letter is. So a
  letter is hit on its ink and not on its cell.

### What it cost

- four tests in the battery — the paragraph is drawn, the letter is as
  wide as the metric says, the letter is hit and the margin is not, and
  the paragraph is framed by its lines — and **two** in the studio.

### What it does not settle

- **The drawing is rebuilt every time.** A paragraph is broken, laid
  out and drawn again for every picture, and a document with many
  paragraphs pays for it every frame. The lines are derived (§58) and
  so are the letters; what a bigger document would want is the
  derivation done **once** per frame.
- **The ruler still has no scale** (§126). The kit has a typeface now,
  so the bar numbers are within reach — and they are not drawn.

---

## 130. The canvas's text is a letter

The canvas drew a character as the **box** it would occupy — a
typewriter's worth of rectangles — and the picture said *there is text
here* and not what it says. That was honest, and it was a placeholder.

### What was built

- The canvas's `text` draws the kit's **own letters**: the typeface is
  a table of strokes (§128) and a character is drawn by its strokes.
- The **cell** is still one for every character, so a line of text is
  still monospaced — but the cell holds a letter and not a rectangle.
- The **baseline** is the point the text is drawn at, and the capitals
  are two thirds of the line height: a line needs room below the
  baseline for the letters that hang there.

### What the port found

- **The letter is stretched to the cell.** The cell says how wide the
  character is and the typeface says what it looks like, so the glyph
  is scaled to fit — the same rule the text battery follows (§129). One
  rule, two places, and the canvas did not need a metric: the cell is
  the metric.
- **The canvas's seam did not change.** It still takes a string and a
  point. What changed is what the **backend** does with them, and the
  backend is the one place a typeface belongs.
- **A box and a letter are different things to a machine.** A box says
  where a character is; a letter says what it is. The kit had the first
  because the second was missing, and the first was a poor substitute
  for the second.

### What it cost

- one dependency, one function rewritten and two constants, and **two
  tests**: a letter is a line and not a box — the ink in its cell is a
  fraction of what a box would cover — and a space takes a cell and
  draws nothing.

### What it does not settle

- **The canvas's text is still monospaced.** The cell is fixed, so a
  narrow letter and a wide one take the same room. A canvas that
  measured its text would want the advance of each character, and the
  seam would need a number it does not have. — **Answered in §132**:
  the seam takes the size, and the advance is the typeface's own.
- **The picture has no anti-aliasing** (§67), so a letter is a
  staircase like everything else. A letter is thinner than a rectangle,
  so it shows the staircase more.

---

## 131. The ruler has a scale: the bar numbers

§126 gave the ruler its **marks** and left it with no numbers — and the
reason was the missing font. The kit has a typeface now (§128), so the
numbers are possible.

### What was built

- **The ruler is a node.** The ruler has a **setting** — the scale it
  counts by — and a setting is a thing in a document, and a node is
  where settings live: the same rule as the measure of a paragraph
  (§58) and the appearance of a node (§95). So `sound.ruler.add` puts a
  ruler in the document, and the scale is **bars** or **seconds**.
- **The bar numbers** come from the **tempo map**: the map says which
  bar a tick is in, and the tick of a bar is found by a **search**,
  because the bars are monotonic in the tick.
- **The numbers are the kit's own letters**, drawn in the row above the
  tracks, standing on the bottom of that row so that the marks of the
  tempo and the meter are above them.
- **Two numbers are never too close.** The gap is in **seconds** and not
  in bars: a tempo may make the bars any width it likes, and the ruler
  is drawn in seconds.

### What the port found

- **A ruler is a view, and a view can have settings.** The ruler is not
  in the sound document's model — the tree this ports has a ruler in its
  window, not in its file — and v2 has no window of its own. So the
  ruler became a node: the one thing a view needs that a model does not
  have is a **setting**, and a node is where settings live. The ruler's
  **marks** stayed where they were — the tempo's and the meter's own
  kinds draw them (§126) — so the ruler draws only the numbers, and the
  two do not draw over each other.
- **A number is a word of one or two letters**, so the whole of the
  ruler's scale is the typeface: no number format, no digits of its
  own, and no table of number shapes.
- **The bars are monotonic in the tick**, which is what makes the search
  possible. The tempo map was written for the opposite question — what
  bar is this tick in — and the reverse question has the same answer
  because the map's segments are in order.

### What it cost

- one kind, one command, two functions in the timeline, and **five
  tests**: the bar numbers, the seconds, the row above the tracks, a
  scale that is not a scale, and a studio test that the numbers are
  drawn.

### What it does not settle

- **The numbers are the ruler's own letters, stretched to nothing.**
  The digits are drawn at the em of the ruler's row and their widths are
  the typeface's, so two numbers that are far enough apart in **time**
  may still be close in **ink** when the bar is short.
- **There is no time signature on the ruler.** The meter is a mark in
  the row (§126) and not a number, so a change of meter is visible and
  not readable.
- **The ruler is drawn once per frame**, like everything else, and a
  document with a thousand bars pays a search per bar (§126).

---

## 132. The canvas's text takes a size, and is proportional

§130 gave the canvas real letters, and the letters came out **one size
for everything**: the canvas had a cell of its own, and every character
took it.

### What was built

- **The seam takes the size**: the height of the capitals — the **em** —
  in the world's units. The size is the caller's, because the size is
  the caller's business.
- **The advance is the typeface's own**, so the text is **proportional**:
  a narrow letter takes less room than a wide one.
- The **CAD battery** passes the text's own **height**, so a text that
  says how tall it is is drawn that tall.

### What the port found

- **A seam that takes a number is a seam that can be changed.** The
  canvas's text had no size because the canvas had a size of its own —
  the fixed cell — and a caller with a size could not give it away. The
  CAD battery's text **has** a height, in its payload, and it was
  ignored: the drawing and the model disagreed and the drawing won.
- **The size and the width are two different things.** The size is the
  caller's; the width of each letter is the typeface's. A metric may
  say otherwise — and then the metric wins, as it does in the text
  battery (§129) — but a canvas's text has no metric, so the typeface
  is the only truth there is.
- **The recording canvas follows the seam.** It records the size too,
  because a recorder that forgets a parameter is a recorder that cannot
  be trusted to tell two drawings apart.

### What it cost

- one parameter on a seam, two implementations, one caller and **two
  tests**: the text is proportional — a line of `i` is narrower than a
  line of `m` — and the size is the caller's, because the big letter is
  bigger.

### What it does not settle

- **A canvas's text has no metric.** The letters are the typeface's own
  widths, so a text that is measured by a **metric** — a paragraph
  (§129) — and a text that is drawn by the canvas are measured
  differently. The two never meet today: the text battery draws its
  letters as paths and the canvas draws the CAD's text.
- **The picture has no anti-aliasing** (§67), so a letter thinner than
  a pixel is a letter that comes and goes with the zoom.

---

## 133. The WAV reader: the samples of a sound, from the file

The kit can draw a mixer and it cannot make a sound (§127), and the
first thing that was missing was a way to get the **samples** in.

### What was built

- **`craftkit-wav`**: the **RIFF** container, the **format** chunk and
  the **data** chunk, read into samples.
- The samples come back as `f32` — interleaved, between −1 and 1 —
  whatever the file's own width was: sixteen, twenty four and thirty
  two bit whole numbers, eight bit unsigned, and thirty two and sixty
  four bit floats.
- The **extensible** format is read too: it says what it is in its own
  sub-format, and the sub-format's first two bytes are the answer.
- The file is **hostile**, so every number in it is checked before a
  single sample is read: the format is one the kit reads, the channels
  are between one and the limit, the rate is not nothing and not too
  fast, the samples are within the limit, and the samples are a whole
  number of frames.

### What the port found

- **A sound file is a header and some numbers**, and the header is the
  whole of the format: the rest is convention about how the numbers are
  packed. So the reader is a small parser with a table of widths and
  not an engine.
- **A file that says one thing and holds another is refused.** The
  samples are a whole number of frames, and a file whose data chunk is
  a half frame short is not read: the numbers would be wrong in a way
  nobody could see.
- **The widths are the same problem the metric had.** A file's width is
  a property of the **file** and not of the sound: the same recording
  is eight bit in one file and sixteen in another, and the reader gives
  back one thing — the numbers — so that the caller never has to know.
  It is the same rule as the metric's (§60), with the file in place of
  the document.

### What it cost

- one crate, one module and **fifteen tests**: the samples of every
  width the kit reads, the interleaving of a stereo file, the
  extensible format, and a refusal for every way a file can lie.

### What it does not settle

- **There is still no audio.** The reader gives back samples and
  stops: there is no engine that mixes them, no device that plays them
  and no playhead that moves. The mixer is a picture of a mixer
  (§127), and now the samples are in the room. — **Answered in §134,
  §135 and §136**: the engine is the mixer, the document is rendered
  into a file, and the playhead says where the sound is. What is still
  missing is a **device** and a **clock**.
- **Only WAV.** A sound file may be in AIFF or in FLAC, and those are
  refusals with names.
- **The file's own extras are left in it.** A WAV may carry a list of
  markers or a name, and the reader does not look for them.

---

## 134. The mixer, and a file the kit can write

The kit could draw a mixer (§127) and could not make a sound, and the
missing pieces were two: the **sum**, and the **file**.

### What was built

- **`craftkit-audio`**: the mixer. It takes **tracks** — a stretch of
  samples that starts somewhere, at a gain — and gives back the sum.
- The gain in **decibels** becomes the gain of a ratio: twenty times
  the logarithm, because a bel is a factor of ten of **power** and
  power is the square of a gain.
- **`craftkit-wav` writes**: a sound it wrote is a file it can read
  back, at sixteen bits, because that is the width every tool reads.
- The **clipped** samples are a decision about the **output**: a mix
  may be louder than one, and a file cannot say how loud it is.

### What the port found

- **A mixer is a graph, and a graph of arithmetic is arithmetic.** The
  kit's mixer is buses, sends, inserts and automation (§20.4) — a
  *graph* of routing — and the graph is the caller's business. So the
  library here **sums**, and it does sums: no bus, no send, no insert.
  That is a smaller thing than a mixer and it is the part of one that
  is arithmetic.
- **A track that starts before the beginning is refused.** A start came
  from a document and a document can say anything, and a track that
  starts at −1 is not a track — it is a number that was not checked.
- **The mixer does not clip.** The sum of two loud tracks may be louder
  than one, and the clip is a decision about the file, so it happens
  when the file is written.

### What it cost

- one crate, one module and **ten tests**: the sum, the gain, the
  length, the decibel, the three refusals, the clip — and the round
  trip, because a sound the kit writes is a sound the kit reads.

### What it does not settle

- **There is no bus and no send.** The sum is the sum; a bus is a track
  that other tracks feed, and a send is a track that is fed from
  another, and neither is built.
- **The mix has one rate.** Tracks are added sample for sample, and a
  track at another rate is a track at the wrong rate. The renderer
  refuses such a source (§135) rather than resampling it. — **Answered
  in §137**: the renderer resamples it.
- **There is no device.** A mix is a buffer of numbers, and something
  has to play it: the kit has no audio device and no playhead.

---

## 135. The document is rendered into a sound

The mixer summed numbers and nothing summed them. Now the **document**
is rendered: the sources it names are read, the clips are the tracks,
the tracks are mixed, and the mix is written to a file.

### What was built

- **`sound.render`**: the renderer. It is **offline** — it writes a file
  and it stops — because the kit has no audio device and no audio
  thread, and a file is a thing a program can make without a device.
- The **clips are the tracks**: each clip plays a source from an
  offset, for a length, at a gain, and the gain in decibels becomes the
  gain of a ratio (§134).
- The **sources are read once** and the clips that play one share it.

### What the port found

- **A renderer is a command and not a thread.** The kit's rule is that
  everything a user can do is a command (§ the registry), and a render
  is something a user asks for: *make me this file*. A thread that
  renders while the window draws is a different thing, and the kit does
  not have it — the **audio thread** is still in the not-built list.
- **The refusals are the interesting part.** A source that is not there
  is refused; a source at **another rate** than the session is refused,
  because there is no resampler here and a resampled clip is not the
  clip that was asked for; sources that do not have the same number of
  channels are refused; and a file larger than the limits allow is
  refused **before a byte of it is read**.
- **The file is the deliverable.** The render does not return a buffer
  to a caller that cannot play one; it writes a file, and the file is
  a sound any tool can open.

### What it cost

- one command, two helpers and **two tests**: the document is rendered
  and the file it wrote is read back and holds the source's samples,
  and a source that is not there is refused.

### What it does not settle

- **There is no device and no playhead.** The render makes a file; the
  kit cannot play it, cannot show where it is and cannot move a cursor
  along it. — **Partly answered in §136**: the playhead is a node and
  it says where the sound is; what is missing is a **device** and a
  clock to move it.
- **The mixer's graph is not rendered.** The buses, the sends, the
  inserts and the automation lanes (§127) are in the plan and not in
  the render: the render sums the clips. — **Answered in §138 and
  §140**: the **automation** is rendered — the lanes move the volume,
  the pan and the mute — and the **routing** is rendered too: a track's
  signal, the sends that carry it into the buses, and the master that
  is the file. What is still missing is the **inserts**: a plugin's
  name is not a plugin.
- **There is no resampler and no time stretch.** A source at another
  rate is refused, and a clip that should be slower than its source is
  not made. — **Answered in §137**: the resampler is linear and the
  render uses it. What is still missing is a **time stretch** — a clip
  that should be slower than its source is not made.

---

## 136. The playhead is where the sound is

The render made a file (§135) and there was no way to say **where in
it** the sound was.

### What was built

- **The playhead is a node**: it has a **setting** — the position — and
  a setting is a thing in a document, and a node is where settings
  live. The same rule as the ruler's scale (§131).
- The position is a number of **samples**, and it is worked out against
  the session's rate: the world is in seconds and the document is in
  samples.
- The playhead is drawn as a **line** from the ruler's row through the
  tracks, because a playhead is where the sound is and the sound is
  under the ruler.
- `sound.playhead.set` **moves** it: there is one playhead, and the
  command moves it rather than adding another.

### What the port found

- **The transport is a place and not an event.** The tree this ports
  has a transport that says "play" and "stop" and moves a cursor; here
  the position **is** the number, and there is no event to catch. That
  is the rule the plugin batteries already follow (§ the position is a
  tick and everything else is derived), and it is the same rule here.
- **A view with a setting is a node.** The playhead is a view thing —
  it is not part of the design — and it has a setting. The ruler made
  the same argument (§131) and the answer is the same: a node is where
  settings live, and a view that is not in the document has nowhere to
  keep its number.
- **The line is drawn from the ruler down.** The playhead crosses the
  ruler's row because the ruler is where the time is, and a cursor that
  stopped at the tracks would say nothing about the time.

### What it cost

- one kind, one command, one place function and **three tests**: the
  playhead is a line at the position, it is moved and not duplicated,
  and a position before the beginning is refused.

### What it does not settle

- **Nothing moves it.** The playhead says where the sound is and the
  kit has no **clock**: there is no thread, no device and no timer, so
  the position is a number a caller sets.
- **The playhead is in the document, so it is in the file.** A document
  that is saved carries the position it was left at, and that is a
  **view** in the design. It is the same trade as the ruler's scale
  (§131): the setting is somewhere, and the document is where the
  settings are.

---

## 137. The resampler

The render refused a source that was at another rate than the session,
because there was no resampler and — the render said it plainly — a
resampled clip is not the clip that was asked for.

### What was built

- **A linear resampler** in the mixer: the sample at a place in the
  new rate is the line between the two samples it lies between.
- The **channels are kept apart**: a frame of the new sound is a frame
  of the old one, so each channel is resampled on its own.
- The **render uses it**: a source at another rate is resampled to the
  session's rate, and the sound is the same sound.

### What the port found

- **A resampler is arithmetic**, and so it is a library: it takes
  samples and a pair of rates and gives back samples. The tree's
  resampler was read as a **plug-in** — a plugin that changes the
  rate — and here it is a function, because the thing that needs it is
  a *render*, and a render is a command. **§155 found that the tree's
  resampler is not a plug-in**: it is a library in the tree's own dsp
  crate, and the reason the port could not have had a better one was
  not that a filter is a plugin but that nobody had gone to look.
- **Linear is honest.** It is not the best resampler and it does not
  pretend to be: a sound resampled by a line has the softness that a
  line has. **§155 replaced the line with a filter**, and the filter
  is a library here for the reason the resampler was.

### What it cost

- one function and **three tests**: the resampler keeps the sound and
  changes its rate, it keeps the channels apart, and the render writes
  a source at another rate at the session's rate.

### What it does not settle

- **There is no time stretch.** A clip that should be *slower* than its
  source is not made: the resampler changes the rate of the sound, and
  a stretch is a change of rate with the time held still, which is a
  different thing.
- **The resampler is linear**, so a sound resampled to a much higher
  rate loses its top. A better resampler is a **filter**, and a filter
  is a plugin. — **§155 settled this**: the filter is here, and it is
  a library and not a plugin.

---

## 138. The automation is heard

The lanes were drawn (§127) and moved nothing: a curve on a track's row
that no sound obeyed.

### What was built

- The **render applies the lanes**, per sample, because that is what a
  curve is: the volume at the sample's own time.
- **Three parameters** are worked out, because a sound is made of them:
  - **volume**, in **decibels** — like the gain of a clip;
  - **pan**, between minus one and one — the balance between the
    channels, and a **mono** sound is not balanced, so a lane of pan
    over a mono track does nothing;
  - **mute** — a value of one or more mutes the track, and a value of
    less than one does not.
- The value **before the first breakpoint** is the first value and the
  value **after the last** is the last one: a lane holds the value it
  has until the lane says otherwise. Between two breakpoints the value
  is the **line** between them.

### What the port found

- **A curve is per sample, and not per clip.** A clip that runs across
  a breakpoint is loud at one end and quiet at the other, and the sound
  says so: the automation is worked out at each sample's own time and
  not at the clip's start.
- **The parameter's unit comes from the parameter.** The volume is in
  decibels because the gain of a clip is in decibels (§ the clip's
  payload) and the two are the same parameter of the same track. The
  pan is a balance because a balance is what a pan is.
- **A lane of an unknown parameter is drawn and is not heard.** The kit
  does not know what the parameter means, and it does not guess: the
  parameter is a **string** (§20.4), so a third party may automate
  anything, and the kit's renderer works out the three that a sound is
  made of.

### What it cost

- one module, one function and **two tests**: a lane of volume is
  heard sample by sample, and a mute lane mutes the track.

### What it does not settle

- **The routing is still not rendered.** The buses and the sends are in
  the plan and not in the sound: the render sums the clips, and a
  track's send to a bus changes nothing that is heard. The model has
  no **master bus**, so what a track outputs to has no answer yet. —
  **Answered in §140**: the master is the file, and the model does not
  need a node for it.
- **The inserts are not rendered.** An insert is a plugin's name (§ the
  insert's payload) and the render does not load plugins. A plugin is a
  **program** (§ the plugin's own section) and a program needs a host,
  and the render is not one. — **Answered in §141**: there is a
  catalogue of the kit's own plugins, and the render runs the inserts
  of a track over its signal.
- **The lane's curve and the lane's sound are two things.** The curve
  is drawn scaled to its row (§127) — the kit does not know what range
  a parameter takes — and the sound uses the parameter's own units. A
  lane that is drawn one way and heard another is a lane whose drawing
  says the shape and not the values.

---

## 140. The routing is heard

The buses and the sends were in the plan, in the picture (§127) and in
no sound: the render summed the clips and stopped.

### What was built

- The render follows the **graph**. A track's **signal** is the sum of
  its clips; a **send** carries a copy of that signal, at its own level
  and its own pan, into the bus it names; and the **master** is the
  file — every track and every bus, added together.
- A send's **pan** is a balance between the channels, and a **mono**
  signal is not balanced and is left alone.

### What the port found

- **The model does not need a master bus.** The obvious reading of a
  mixer is that a track feeds a master and its sends feed the buses,
  and that the master is a thing in the document. It does not have to
  be: a track feeds the master, its sends feed the buses, and the buses
  feed the master — so the **master is the file**, and a document that
  says nothing about a master says everything about where the sound
  goes.
- **A send is a copy, and not a move.** The track's signal is heard on
  its own **and** through the bus, at the send's level: that is what a
  send is, and it is why the render adds the bus to the master and not
  instead of the track.
- **The graph is one level deep.** A bus is fed by the sends of the
  tracks and feeds the master, and there is no bus that feeds a bus —
  so the order of the buses does not matter to the sound, and the
  routing's own order (§ the routing) is about the *plan* and not about
  the samples.

### What it cost

- three helpers — the sends, the sum and the pan — and **one test**: a
  track that sends to a bus at six decibels down is heard twice, once
  on its own and once through the bus.

### What it does not settle

- **The inserts are not rendered.** An insert is a plugin's *name* and
  the render does not load plugins: a plugin is a **program** (§ the
  plugin's own section) and a program needs a host, and the render is
  not one. — **Answered in §141**: the catalogue is the host's own
  small one.
- **There is no master of its own.** The file is the master, so a
  document cannot have two outputs and cannot say that one bus is a
  master and another is not.
- **The pre-fader flag does nothing.** A send says whether it is before
  the fader, and the model has no fader: the tree's mixer has one and
  this one does not, so both kinds of send carry the same signal.

---

## 141. The insert is a program

An insert named a **plugin** and no plugin was ever run: the tree's
mixer has a table of thirty built-in plugins and a hard-coded match on
the id, and v2 had the name and nothing else.

### What was built

- **`craftkit-plugins`**: a catalogue of the kit's own **audio
  plugins** — a gain, a one-pole low pass and a delay. A name and a set
  of parameters make one.
- The **render runs the inserts**: a track's signal goes through them,
  in the **order of the tree**, because the order of the tree is the
  order of the drawing and it is the order of the processing too.
- The insert's parameters could not be set at all before this, so there
  is a **command** for it: `sound.insert.set`. A document may not
  change its own parameters; the change is a command, like every other
  change.

### What the port found

- **"Plugin" means two things in this kit, and they are two different
  things.** A **plugin of the registry** is a crate that contributes
  kinds and commands; a **plugin of the audio** is a program that
  changes a signal. The first is wired in at build time by the linker
  (§ the linker); the second is named by a document and made at run
  time, and the catalogue is where the names are. Writing both in one
  crate would have hidden that they are different questions.
- **A plugin that is not in the catalogue is refused.** The tree's rule
  is that a plugin that is not in the table cannot be created at all,
  and here it is a refusal with a name: a plugin that cannot be made
  cannot be run.
- **The parameters are hostile, like everything else.** A parameter that
  is not a number is **left out** — the reader's rule — and a
  parameter that is not a **size** is refused: a cutoff of nothing is
  not a cutoff, and a delay of minus five is not a delay.

### What it cost

- one crate, one trait, three plugins and **eight tests** in the
  catalogue, and **two** in the battery: the gain is a gain, the low
  pass is smooth, the delay is later, a name that is not in the
  catalogue is refused with its name, and an insert changes the sound
  it is on.

### What it does not settle

- **The catalogue is three plugins long.** The tree has thirty, and
  they are the tree's own code; a catalogue is a place to put them, and
  what is in it is a decision about which are worth porting.
- **A plugin here is not a plugin there.** The kit's CLAP and VST3
  batteries carry a **host** that opens a real plugin from a real file
  (§76–§120), and those plugins are programs in a shared library. The
  catalogue here is programs in a source file: they are made at run
  time from a name, and they cannot be anything but what the tree
  wrote.
- **The low pass is one pole.** It is the simplest filter there is that
  changes a signal, and it is honest about what it is; a better filter
  is more poles and a filter with a state.

## 142. The picture is drawn

A picture was a **size and a file** and nothing ever looked at the file:
the image battery could measure a picture and could not show it. The
pixels were in the file, and the file was nowhere.

### What was built

- **`Picture`** in the SDK: the pixels of an image — four bytes a pixel,
  red, green, blue and alpha — and the rows top to bottom. It is in the
  SDK and not in a rasteriser because the **canvas** is the seam that
  draws it: a backend that draws a document may be a raster, a PDF
  writer or a recorder, and all of them need the pixels in the same
  shape.
- The **canvas** gained a `picture` method: a picture is drawn at a
  rectangle and scaled to fill it, the nearest pixel taken, and an alpha
  blended over what is under it.
- The image battery's **paint** reads the file, decodes it — the PNG
  reader and the JPEG decoder are the tree's own — and puts the pixels
  on the canvas.

### What the port found

- **A picture is a file, and the file is read when it is drawn.** That
  is a cost every frame, and it is the honest one: a document that held
  the pixels would be a document of a different kind, and the bytes
  travel beside the document in the bundle (§25) for exactly this
  reason.
- **The canvas draws what the caller gives it.** The picture arrives as
  pixels and is drawn where the node says; nothing in the drawing
  decides for itself.
- **A picture with no pixels is not drawn at all.** Half a picture would
  be a picture of the wrong thing, and a picture that is not all there
  is a picture that is not there.

### What it cost

- one type in the SDK with **five tests**, **three** in the rasteriser
  and **six** in the battery: a grey picture is grey, a colour picture is
  opaque, an alpha is kept, a pixel that is not there is not there, the
  red half of a picture is red and the blue half is blue, the outside is
  not painted, a half-transparent picture is half over what is under it,
  a picture with no pixels draws nothing, the picture knows its size,
  and a picture that is not there is refused.

### What it does not settle

- **The picture is read again for every frame.** A document that is
  painted sixty times a second reads its file sixty times a second. The
  fix is a cache, and a cache is a thing that can go stale, and a
  picture that is stale is a picture that is wrong.
- **Only the formats the tree can read are drawn.** The PNG reader and
  the JPEG decoder are here; a TIFF or a WebP is not, and a picture in
  one of those is measured by its header and not drawn.

## 143. The clip is stretched

A clip could be placed, trimmed and given a gain, and there was no way
to say that it should be played at another speed. The family's clips
carry one — *"Elastic / Vari-speed ratio (1.0 = none) — applied by
rendered processing"* — and v2's did not.

### What was built

- **`varispeed`** in the audio crate: a sound played at another speed.
  The factor is a **length ratio**, the reading is **linear** — the
  sample at a place in the new sound is the line between the two
  samples it lies between — and the channels are kept apart.
- The clip's **payload** carries a `stretch`: one is none, and it is a
  number in the document, so it is saved, undone and shared like any
  other part of the clip.
- **`sound.clip.stretch`**: the command that sets it, and
  `sound.clip.add` takes it as well.
- The **render** honours it: the source is read more slowly, so the
  sound is longer and lower. The clip keeps its place and its length —
  the stretch says *how the source is played*, and not how long the
  clip is.
- The **trim** honours it: the content moves by the delta **divided by
  the stretch**, exactly as the family's trim does.

### What the port found

- **A stretch is a varispeed, and not a time stretch.** The difference
  is the whole point: a time stretch keeps the pitch and changes the
  length, and that is a WSOLA and much harder to do well. v2 does the
  varispeed and says so in the name of the function.
- **The picture and the sound disagreed about a clip's length.** The
  timeline drew the clip's length in the **session's** samples and the
  render measured it in the **source's** frames, so a source at
  another rate was drawn one length and heard another. The payload now
  speaks the session's samples — the timeline's language — and the
  render works out the difference where it reads the source. One test
  changed its mind, and it was the test that was wrong.
- **The stretch is one fact and it says two things.** The clip is
  longer and the content moves more slowly: those are the same
  number, and the trim uses it as `delta / stretch`.
- **A clip that cannot be read is refused, and not left out.** The
  render used to skip a clip whose payload it could not parse, which
  is a sound that is missing something without saying so.

### What it cost

- one function with **four tests** in the audio crate, one field in the
  payload, one command, and **ten tests** in the battery: a clip
  carries its stretch, a stretch is set on a clip, a clip may be added
  already stretched, a stretch that is not a number is refused, a
  stretch of zero is refused, a stretch that is too large is refused, a
  stretched clip is longer and lower, a stretch of one is no stretch, a
  trimmed stretched clip moves its content by the delta over the
  stretch, and the picture and the sound agree.

### What it does not settle

- **The reading is a line.** A varispeed read by a line is not a
  varispeed read by a good filter: the top of the sound is a little
  rough, and a sound played very slowly is very rough.
- **The stretch is bounded at 64.** A clip stretched more than that is
  refused, and the bound is a number in the tree and not a law.
- **There is no time stretch at all.** A clip that keeps its pitch and
  changes its length is a WSOLA, and it is not here.

## 144. The clip fades

A clip could be placed, trimmed, given a gain and stretched, and there
was no way to say that it should come up slowly or go down slowly. The
family's clips carry a fade at each end, and v2's did not.

### What was built

- **`FadeShape` and `Fade`** in the audio crate: the curve of a fade —
  linear, equal power, S-curve, exponential, logarithmic — and a fade
  is a length, a shape and a **bend**. The bend pushes the curve away
  from itself, and its ends stay where they were, so that a fade still
  begins at nothing and still ends at full.
- **`faded`**: the samples with a fade at the head and a fade at the
  tail. The channels are kept, and a fade is a gain and not a thing
  that belongs to a channel.
- The clip's **payload** carries a `fade_in` and a `fade_out`.
- **`sound.clip.fade`**: the command that sets them. The shape and the
  bend are read once, and they apply to the fades that are being set.
- The **render** honours the fades: the head of the clip comes up and
  the tail goes down.
- The **picture** honours them too: the quiet part of a clip is a
  **notch** in its bar — the paper showing through the ink, and the
  notch is the fade's own curve.

### What the port found

- **A fade is one curve and two ends.** The tail's curve is the head's
  curve read backwards, because the silence is at the end, and the
  gain at the last sample of a fade of two is a half of the way down
  and not nothing: the curve reaches nothing one sample later.
- **The bend is a weight and not an offset.** The family's bend pushes
  the curve towards the middle and leaves its ends alone, and the
  port kept that: a bend of one makes a linear fade steeper in the
  middle, and a bend of minus one makes it flatter.
- **A fade that does not fit is refused.** The family clamps a fade to
  the length of the clip it fades; v2 refuses it, because a fade that
  is quietly shortened is a fade the caller did not ask for.
- **The quiet part of a clip is a hole in its bar.** Drawing the fade
  needed a colour that is not ink, and the paper is the only one the
  kit has.

### What it cost

- one type and one function in the audio crate with **seven tests**, one
  field in the payload, one command, and **ten tests** in the battery: a
  fade shape is a curve from nothing to full, a fade shape is read by
  its name, a bend pushes the curve away from itself, the samples are
  faded at the head, the samples are faded at the tail, a fade of no
  length changes nothing, a fade keeps the channels apart, a clip
  carries its fades, a fade is set on the clip, a fade longer than the
  clip is refused, a fade with no length is refused, a shape that is not
  a shape is refused with its name, a faded clip starts quiet, a faded
  clip ends quiet, the default shape is the equal-power one, the fade is
  drawn, and a clip with no fade is drawn with no notch.

### What it does not settle

- **The curve is drawn with eight segments.** A notch in a bar is
  small, and the notch is a hint and not the sound.
- **Equal power is the default, and equal power is right for a pair of
  fades and not for every pair.** The family's default is the same,
  and the default is a choice and not a law.
- **A fade is on a clip and not on a source.** A source played by two
  clips is faded twice, once for each clip, and a fade that belongs to
  the source would be a different thing.

## 145. The word is broken

The line breaker knew a **hyphenation point** — a flagged penalty that
adds a hyphen's width when it is taken — and nothing could say where
the points were. The caller had to, and the caller had no dictionary,
so no paragraph in the tree could be hyphenated at all.

### What was built

- **`craftkit-hyphen`**, a library: the **patterns**, the **limits**
  and the **heuristic**.
  - The **patterns** are a set that says where the words of a language
    break. They are written in the notation the family uses — a word
    with digits in it, and a digit says how many letters are on that
    side of the break — and where many patterns meet, the **biggest**
    digit wins.
  - The **English set** is in the tree as **source**: a few thousand
    patterns in a file of its own, because the way this kit holds data
    is as code.
  - The **heuristic** answers for the words the patterns do not know: a
    word is split into syllables by rule, and the rule is right often
    enough to break a line with.
  - The **limits** say how careful the answer is: how short a word
    must be, how many letters must stay at each end, and whether a
    word in capitals may be broken at all.
- The **text battery** asks the kit: the paragraph's payload carries
  `hyphenate`, and **`text.paragraph.hyphenate`** sets it. A paragraph
  is not hyphenated until somebody says so — a hyphen in a line is a
  thing a reader sees.
- The **breaker** gets the points as **hyphens**: a broken word is
  more than one box, and the hyphen between the pieces is a break
  opportunity that costs a hyphen's width.
- The **drawing** draws the hyphen, because the typeface has had one
  all along and a line that ends at a hyphenation point ends in a
  hyphen.

### What the port found

- **The patterns are data, and this tree has no assets.** The family
  holds its set in a file; this tree holds it in source, and the
  reason is the rule the tree already follows: a program that reads a
  file it ships is a program with an asset, and a pattern set is data
  the program reads.
- **The trained set is not the thesis's set.** Liang's thesis breaks
  "hyphenation" as *hy-phen-ation*; the trained set breaks it as
  *hy-phenat-ion*. Both are English, and the port keeps the trained
  one: a set trained from a word list knows more words than a set
  written by hand.
- **A broken word is more than one item.** The breaker's items were
  boxes and glue in strict alternation, so a word's item index was
  twice its word index — and a word that may be broken breaks that
  arithmetic. The lines are now reported through a map of which word
  each item belongs to, and the picture draws the **pieces**.
- **The hyphen was in the typeface all along.** The picture did not
  show it because nothing drew it: a line that ended at a
  hyphenation point ended in nothing.

### What it cost

- a new crate with **eleven tests**, one field in the paragraph's
  payload, one command, and **six tests** in the battery: the patterns
  break a word where the thesis says, the trained set breaks it in its
  own way, a word is broken by the patterns, a short word is not
  broken, a break leaves two letters at each end, the limits are the
  caller's, a word in capitals is not broken, a capital word may be
  broken, a capital word may be kept whole, an apostrophe is not a
  break, a word with an unknown letter is split by the heuristic, a
  paragraph is not hyphenated until it is asked to be, a hyphenated
  paragraph breaks the word, the line that ends at a hyphenation point
  is flagged, the words of a line are the words of the paragraph, the
  hyphen is drawn, and a node that is not a paragraph is refused.

### What it does not settle

- **The set is English, and it is one set.** The family has Polish,
  Portuguese, Italian, German and Bulgarian, and a rule for Turkish;
  none of them is here, and the engine is the same for all of them.
- **There is no dictionary.** The family's first answer for a word is
  its dictionary — a hundred and sixty thousand words — and this tree
  has none: the patterns answer, and the heuristic answers what the
  patterns do not know. A word whose spelling is unusual may break in
  the wrong place, and the caller may still say where a word breaks.
- **The limits are the kit's and not the paragraph's.** The library
  takes the limits from the caller and the paragraph uses the
  defaults; a paragraph cannot yet say that it wants three letters
  left at the end.

## 146. The click is heard

A sound editor can play a sound and cannot count it out. The family
has a **metronome** — a click on every beat, and a different click on
the first beat of a bar — and this tree had no way to say that a
document should have one.

### What was built

- **`blip`** in the audio crate: a sine at a pitch, under an
  exponential envelope, added to the samples and to every channel. It
  lasts thirty thousandths of a second, which is long enough to be
  heard and short enough to be a click.
- **`midi_to_hz`**: a note is a number and a frequency is 440 hertz
  times two to the power of the semitones, and an octave is a
  doubling.
- The **session's settings** carry the metronome: whether it is on,
  how loud it is in decibels, the click's note and the **accent's**
  note. `sound.session.set` sets them.
- The **render** mixes the click in **after** the master, so it reaches
  the output without passing through a track, a send or an insert.
- The **beats** come from the tempo map and the **meter** says how long
  a beat is, and the first beat of a bar is the accent.

### What the port found

- **The click is a voice of its own.** The family sums it in after the
  master fader, so it goes to the speakers without passing through any
  track, send or insert — and a track that is silent does not silence
  the click.
- **A click is a sine and a decay.** The envelope is what makes it a
  click and not a tone, and the envelope is exponential: a click fades
  away and does not stop.
- **The family never prints the click into a file.** Its rule is that
  an offline render leaves the click off, because the click is for the
  player and not for the record. This tree has no playback engine — it
  has a render — so the click here is printed when the document says
  so, and that is the difference to name.

### What it cost

- two functions in the audio crate with **three tests**, four settings,
  and **five tests** in the battery: a note is a frequency, a blip is
  added to the samples, a blip goes to every channel, the click is not
  heard unless it is asked for, the click is heard on the beat, the
  accent is on the first beat of the bar, the click does not pass
  through a track, and the level of the click is the caller's.

### What it does not settle

- **The click is printed into the file.** The family keeps it out of
  every offline render, and this tree has no playback engine to keep
  it out of: the click is in the file when the document asks for it.
- **The sound is as long as its content.** The click is placed by the
  tempo map, but the file is as long as the clips: a document with no
  clips has no click, because there is no sound to put it in.
- **There is one click and one accent.** The family has a velocity for
  each of them and a "only while recording" flag; neither is here.

## 147. The clip has a curve of its own

A track had a curve and a clip had a gain, and the family's clip
carries a curve of its own — a gain envelope, measured from the clip's
own start.

### What was built

- **A lane automates one thing: a track or a clip.** `sound.lane.add`
  takes a `track` or a `clip`, and exactly one of them: two owners is
  two lanes, and a lane with two curves is a lane that means two
  things.
- The **render** applies the clip's own lanes to that clip, and their
  times are measured from the **clip's** start.
- The **picture** draws the clip's curve on the clip's bar, at the
  clip's place: a clip's curve is drawn where the clip is and not
  where the track's timeline is.

### What the port found

- **A lane belongs to the node it automates.** The tree's lane was a
  lane of a track, and the family's clip carries a curve of its own;
  the tree's answer is that a lane is **under** its owner and that the
  owner may be a clip.
- **The times are the owner's.** A track's lane is measured from the
  track's timeline and a clip's from the clip's own start, and the two
  are drawn where they belong. One lane, one owner, one clock.
- **The curve is a node and not a number in the payload.** The family
  keeps the envelope in the clip's payload; this tree keeps it in a
  lane, because a curve is a thing with a place and a shape, and the
  payload is for the things that are only numbers.

### What it cost

- five tests in the battery: a lane may automate a clip, a lane
  automates one thing, the clip's curve is heard, the clip's curve is
  drawn on the clip's bar, and a track's lane is still the track's.

### What it does not settle

- **The clip's curve multiplies the track's.** The two are separate
  gains and both are applied; a curve that is meant to *replace* the
  track's is not a thing here.
- **Only volume, pan and mute are heard.** A lane of any other
  parameter is drawn and not heard, and that is true of a clip's lane
  as much as of a track's.
- **A clip's curve is one lane per parameter.** Two lanes of volume
  under one clip are two curves and the second wins, and nothing says
  so.

## 148. The formula reads another sheet

A grid's formulas could read the sheet they were in and no other, and
the family's formulas read across the sheets of a workbook: `Sheet2!A1`
is a cell, and it is a cell in **another** sheet.

### What was built

- A formula may name a **sheet**: `Sheet2!A1`, and a name with a space
  in it is written in quotes: `'My Sheet'!A1`.
- A **range** of another sheet may be read, and a function may take it:
  `SUM(Sheet2!A1:A3)`.
- The **workbook**: the sheets of a document, by name. The value of a
  cell is a fact about the **workbook** and not about the sheet: the
  evaluation moved there, because a formula that reads another sheet
  needs the book.
- The **cycle** is a cycle of the workbook: a formula in one sheet that
  reads a cell in another that reads it back is a cell that reads
  itself, and it is refused where it always was refused — at the point
  the formula is set.
- The **precedents** name their sheet, and the **dependents** of a cell
  are the cells that read it, in any sheet of the workbook.

### What the port found

- **A cell's value is a fact about the workbook.** The sheet's own
  cells were enough while a formula could read nothing else; a formula
  that reads another sheet needs the book, and the book is where the
  evaluation went.
- **The cycle was a cycle of a sheet and is a cycle of a workbook.** The
  refusal has not moved and it sees more: the cells being computed on
  the way are named with their sheet, so a message says which sheet
  reads itself.
- **A range is in one sheet.** The family's notation may name the sheet
  at both ends of a range (`Sheet1!A1:Sheet1!B4`), and the port keeps
  that; a range that names two *different* sheets is not a range, and
  it is refused.

### What it cost

- seven tests in the battery: a formula reads a cell of another sheet, a
  quoted sheet's name is read, a range of another sheet is read, a
  sheet that is not there is an error, a cycle across the sheets is
  refused, the precedents name the sheet, and the dependents of a cell
  are the cells that read it.

### What it does not settle

- **The sheets are named and not numbered.** `Sheet2!A1` says the
  sheet's name, and a name is a name: two sheets with the same name are
  one sheet as far as the workbook is concerned.
- **A sheet's name is not checked at the point the formula is set.** A
  formula that names a sheet that does not exist is set happily and
  says so when it is calculated.
- **The formulas are still small.** Five functions, arithmetic and
  comparisons: no dates, no text functions, no lookups and no
  conditional expressions.

## 149. The mix does not allocate

The lane's rule is that its work must not allocate, and the tree said
plainly that the rule was a rule and not a measurement: the work it was
written for — the **mixing** — was not in the lane.

### What was built

- The audio crate's **`Track` owns its samples**. A track that borrowed
  its samples could not be handed to a lane whole, and the mixing that
  may run on a lane may not borrow.
- **`mix_into`**: the mixing, **into a buffer the caller owns**. It
  allocates nothing, it waits for nothing and it does bounded work, and
  `mix` — the one that hands back a mix — is built on it.
- The sound battery's **`MixJob`**: the mix of a document as a **lane
  job**. The tracks are prepared on the control thread — the sources
  are read, the clips are prepared, the tracks' signals are built — and
  the lane does the rest: the block's samples are the sum of the
  tracks, block after block, and nothing is allocated while it happens.
- The lane's work may be **had back**: `LaneJob` is `Any`, so a caller
  may take the work when the lane stops and read what it made. A lane
  that swallowed its job would be a lane that swallowed the samples.

### What the port found

- **The rule was a rule, and the work it was written for is the
  mixing.** The tree's own design document said the measurement had not
  been made because nothing needed it — and now something does: the
  mixing is measured on a **real lane thread**, and the number is
  **zero**.
- **A track that borrows its samples cannot cross the thread.** The
  family's mixer owns its buffers, and the port now says why: the
  buffers are made on the control thread and the lane only reads them.
- **The lane erased its own work.** The seam handed the job back as a
  `Box<dyn LaneJob>`, and a caller that could not name the job could
  not read the samples it had paid for. `Any` gives the work back.

### What it cost

- three tests in the studio: the mixing does not allocate on the lane
  thread, the mix is the sum of the tracks, and the work is had back
  when the lane stops.

### What it does not settle

- **There is no device.** The tree's own seam says so, and the job is
  driven by the caller: the blocks are run and the samples are read.
- **The inserts are not in the lane.** A track's inserts are processed
  over the whole signal, and the family's plugins process a **block**,
  so a document with inserts is not lane-ready yet.
- **The click is not in the lane.** The metronome is mixed in after the
  master, in the render, and a lane that plays a document with a
  metronome would have to mix the click block by block.

## 150. The track is soloed

A track could be muted and could not be soloed, and the family's mixer
silences the other tracks when one of them is soloed: the solo is the
mixer's one rule that is about the *other* tracks.

### What was built

- The track's **payload** carries a `solo`, and `sound.track.solo` sets
  it. The solo is in the document, and a payload is what is saved and
  what is shared.
- The **render** honours it: when a track is soloed, the others are
  silent — and their sends carry the silence, because a send is a copy
  of the track's signal.
- The **picture** honours it: a soloed track's row is a **double
  rule**, so the timeline says which track is heard alone.

### What the port found

- **The solo is a rule about the other tracks.** A mute is about the
  track it is on; a solo is about every track that is not it, and that
  is why the rule lives in the **mix** and not in the track: the track
  says "I am soloed" and the mixer works out what that means.
- **The sends follow.** The family's sends take the track's signal, and
  a silenced track's send is a copy of silence — so the solo does not
  need a second rule for the sends. The port found that when it looked
  for one.

### What it cost

- six tests in the battery: a soloed track silences the others, a track
  that is not soloed is heard with the others, unsoloing a track puts
  the others back, the solo is in the document, the soloed track is
  drawn as a double rule, and a node that is not a track is refused.

### What it does not settle

- **There is no "solo safe".** The family's track has a flag that says
  it is heard even when another track is soloed, and there is no such
  flag here.
- **There is no solo mode.** The family has a setting for what a solo
  does to the tracks that were already soloed — the classic "soloing
  the second track mutes the first" — and there is no setting here: a
  soloed track is heard until it is not.
- **The solo is in the file.** The family's click is not printed into a
  file (§146) and its solo is a playback rule; here the solo is in the
  document and the render honours it, which is the same difference and
  the same answer.

## 151. The caret is in the text

A paragraph could be read and could not be edited: there was no caret,
so there was no place to type at and no way to say where in the text
the reader was.

### What was built

- A **caret**: a node of kind `text.caret`, a **child** of the
  paragraph. Its payload carries the **character** it stands before.
- Four commands: `text.caret.set` puts the caret where it is asked
  for, `text.caret.move` moves it a character at a time or to the ends
  of the text, `text.caret.type` types at it, and `text.caret.delete`
  takes the character before it away.
- The caret's **place** is derived and not stored: the drawing places
  every character of the text as it draws it, and the caret stands
  where the character it stands before stands. The place is the kind's
  **place**, so a front end can ask where the caret is.
- The caret is **drawn**: a line at that place, in the text's colour.

### What the port found

- **The caret is a node and not the session's business.** The family's
  caret is part of the selection, and the selection is the session's
  (§22). But a document that keeps the caret keeps where the reader
  was, and the tree's own rule — a view with a setting is a node, the
  ruler's scale (§131), the paragraph's measure (§58) — says the same
  thing. So the caret is a node under the paragraph, and the
  selection is still the session's.
- **The space is a character.** The text is the words with a space
  between them, and the space is a character of the text: it has no
  ink and it is still a place a caret can be. The port found that when
  it tried to place the caret from the letters it had drawn — a letter
  is a letter, and a space is not one.
- **The character is counted in the text, and the text is the words.**
  The words are the payload and the text is what they say, so a
  caret's character is a number of characters in a text that is made
  when the words are made.
- **Typing is the text, and the words are made again out of it.** The
  paragraph does not keep a string: typing puts the characters into
  the text and the text is split into words again. A letter in the
  middle of a word makes the word two words, and a space ends a word
  — that is the payload's rule and not the command's.

### What it cost

- **Fourteen tests** in the battery (`tests/caret.rs`): the caret is a
  node under the paragraph, it is put where it is asked for and moved
  a character at a time, it stops at the ends of the text, a caret
  past the end and a way that is not a way are refused, typing puts
  the text into the paragraph and makes the words again, deleting
  takes the character back, the caret stands where its character
  stands, the caret is drawn, the paragraph says where its caret is,
  and the undo takes the typing back with the caret.
- **The text is bounded at 65 536 characters** (`MAX_TEXT_CHARACTERS`,
  a private cap). The caret's character is a **number**, and the
  payload's numbers are bounded to a million, so a text longer than
  that could not hold a caret in it at all; sixty-five thousand
  characters is far more text than a paragraph has.
- **The drawing now places every character of the text.** The walk
  over the broken lines already knew which word and which piece each
  box belongs to — that is how the words of a line are reported
  (§58) — and the caret wanted the characters as well. The hyphen is
  **not** a character of the text: the word was broken, and the break
  is the breaker's business.

### What it does not settle

- **There is no selection of characters.** The SDK's selection is a
  node and a part of it in the node's own units (§22), and a
  paragraph's own unit is not yet the character. A selection of
  characters would be the caret and the selection in one, and here the
  caret is a node of its own.
- **There is no word-wise movement.** The caret moves a character at a
  time; the family's caret moves a word at a time and a line at a
  time.
- **The caret does not blink.** A caret that blinks is a caret whose
  drawing depends on the clock, and this tree has no clock — the same
  reason the click is not printed into a file (§146).
- **A paragraph with a caret is laid out twice for every frame**: once
  for the letters and once for the caret's line. The port measured the
  cost as small and left it, and that is a measurement and not a
  proof.
- **A caret at a hyphenation break stands at the start of the next
  line.** The hyphen is drawn at the end of the line and the text
  continues on the next one, and the caret stands where the next piece
  starts.

## 152. The part is picked out

§151 said that there was no selection of characters, and the reason was
in the core: the selection holds a **part** of a node, and the core had
no way to ask a kind where that part is.

### What was built

- A **seam**: a kind may say where the picked-out **part** of its node
  is. The core asks, the way it asks where the node is, and the answer
  is a **list** — a part may be in more than one piece.
- The **chrome** marks the part: the picked-out piece is **washed
  over**, and it has no resize handle, because a part cannot be
  resized. A kind that cannot place its part leaves the node's frame to
  say that something is picked out.
- The **paragraph** answers for its characters: a paragraph's own unit
  is the character, so the SDK's own span selection picks out
  characters, and the paragraph says where they are.

### What the port found

- **The core was right and the seam was missing.** The selection has
  held a part of a node since §22, and the part is in the node's own
  units — the very bargain the place is asked under. What was missing
  was the *question*: the core had no way to ask, so it drew the
  node's frame for a part it could not place, and the sound
  application's edit range was a frame around a clip rather than a
  range.
- **A part is in pieces, and the pieces are the point.** A range of
  characters that crosses a line is two boxes, not one box around two
  lines: the part is what a reader picks out, and a reader picks out
  one line's worth at a time. So the seam answers with a list, and the
  chrome washes over each piece.
- **A paragraph's own unit is the character — and the SDK's span
  selection was already the right command.** Nothing new was needed to
  select characters: `select.span` holds a range in the node's own
  units, and the paragraph says what its units are. The port found that
  when it looked for a text-specific command and did not need one.
- **The selection is still the session's, and the caret is still the
  document's.** The two are different things and the port left them
  different: the caret says where the reader *is* and it is saved with
  the text; the selection says what is *picked out* and it is the
  screen's. The family's caret is the selection, and here it is not.

### What it cost

- **Seven tests in the SDK** (`crates/craftkit-sdk/tests/part.rs`): a
  picked-out part is washed over in as many pieces as the kind says,
  a whole node is framed and has a handle, a kind that cannot place
  its part leaves the frame, the picture of the document has no
  selection in it, the core asks the kind, a kind that says nothing
  has no part, and the core does not match on the kind.
- **Six tests in the battery** (`crates/craftkit-plugin-text/tests/selection.rs`):
  the characters are picked out with the span selection, they are where
  they are, a range that crosses a line is more than one piece, they
  are washed over, the picture of the document has no selection in it,
  and a range that is not a range of characters is not a part.
- **One field on the kind's spec**, and one flag on the derive.

### What it does not settle

- **The caret and the selection are two things.** A reader who picks
  out a range has a caret at one end of it in every word processor, and
  here the two are separate: the caret is a node and the selection is
  the session's. Making them one is a change to what the caret *is*.
- **The wash is over the text, not under it.** The chrome is drawn
  after the document, so the picked-out characters are greyed rather
  than highlighted. The order of the drawing is the order of the tree
  (§13), and changing that order is a change to the core's contract.
- **The wash is a fixed colour.** The core has no notion of a
  selection colour, and the appearance of a node is the node's own
  (§95), so the wash is the core's and not the document's.
- **A part is a range and not a set.** The selection holds one range
  per node, so two ranges in one paragraph are two selections and the
  second replaces the first.

## 153. The selection is replaced

§152 said that the caret and the selection were two things and that a
reader who picks out a range has a caret at one end of it in every word
processor. The two are still two things — but an **edit** reads both.

### What was built

- **Typing writes over what is picked out.** `text.caret.type` reads
  the session's selection: a range of characters in this paragraph is
  replaced by what was typed, and the caret stands after it.
- **Deleting takes away what is picked out.** `text.caret.delete`
  takes the picked-out range away instead of the character before the
  caret, and the caret stands where the range began.
- The selection is **gone** after the edit: what was picked out has
  been written over or taken away, and nothing is picked out any more.

### What the port found

- **An edit reads two pieces of state, and that is not a second path.**
  The caret is the document's (§151) and the selection is the
  session's (§22), and the command reads both — which is the same
  bargain the sound battery's trim makes, where the range comes from
  the selection (§ the sound's edit range). The rule that makes it
  safe is the same: the *document* is changed in one place, by one
  command, and the state that says where the reader is is read, not
  written.
- **An empty range is not a picked-out range.** The family's caret
  *is* the selection — a selection with one end — so a range that
  starts and ends in the same place is a caret. Here the caret is a
  node and the selection picks things out, so an empty range picks
  nothing out and the typing goes at the caret. The port found that
  when it asked what a zero-width range should mean and could not
  answer it with one rule for both.
- **Writing over a range is one rule and not two.** The range is
  replaced by the text: the characters of the range go and the typed
  characters come, and the words are made again out of the text. The
  port found that the range and the caret could be **one** rule — the
  same rule with an empty range — once the text was the thing being
  edited.

### What it cost

- **Five tests in the battery** (`crates/craftkit-plugin-text/tests/replace.rs`):
  typing writes over what is picked out, typing with nothing picked out
  types at the caret, an empty range picks nothing out, deleting takes
  away what is picked out, and the selection of another paragraph is not
  this one's.
- **One helper** that reads the picked-out characters, and the text's
  insert-and-delete became **replace-and-delete**: a range that is
  empty is an insertion.

### What it does not settle

- **The caret does not move to the end of a picked-out range.** In the
  family, picking out a range puts the caret at one end of it. Here
  the caret stays where it was until the edit moves it, so a reader
  who picks out a range and then types has the text replaced and the
  caret after it — but a reader who picks out a range and *looks* has
  a caret somewhere else.
- **The selection is the session's and the caret is the document's, so
  the undo restores one and not the other.** Undoing a typing brings
  the text and the caret back, and the selection — which was cleared —
  stays cleared. That is the honest difference between the two homes,
  and the port left it.

## 154. The clip is drawn as its sound

A clip on a timeline was a bar of ink: it said where a sound was and
not what the sound was. A reader could not see a kick from a pad, and
the tree this ports draws every clip as the **shape** of its sound.

### What was built

- The **peaks** of a sound: the extent of the samples in each of two
  hundred and fifty-six buckets — a least and a most each. The peaks
  are the shape of a sound in a small table.
- A **command** that measures them: `sound.source.peaks` reads the
  source's file and puts the peaks — and the file's length, its
  channels and its rate — in the source's payload.
- The **waveform** is drawn: a clip's bar carries the shape of the part
  of the source the clip plays, in the paper's colour, the way the
  fade's notch is.

### What the port found

- **The peaks are a measurement, and a measurement is a command.** The
  tree's own rule for a picture is that the file is read when it is
  drawn and not kept (§ the picture is a file). A sound is a file too,
  but a sound is a great many numbers and a bar is a few: reading a
  sound at every frame is reading it a hundred times a second. So the
  shape is **measured once** and kept in the document — and the
  measurement is undone with everything else and saved with everything
  else.
- **The peaks are the source's and not the clip's.** A source is a
  file, and the file has a shape whatever plays it; the clip says
  which part of it it plays. That is the same bargain as the picture's
  dimensions (§ the picture is drawn): the file's facts live with the
  file's node.
- **A stretched clip plays less of its source.** The clip's length is
  in the session's samples and the source speaks in its own frames,
  and a stretched clip reads its source more slowly — so the shape on
  the bar is the part of the source the clip really plays, and not the
  whole of it.
- **The gain is part of the shape.** A clip that is six decibels down
  is drawn lower, because a reader looks at a clip to see how loud it
  is.

### What it cost

- **Five tests in the battery** (`crates/craftkit-plugin-sound/tests/waveform.rs`):
  the peaks are measured from the file, a sound shorter than the table
  is silence for the rest of it, a clip is drawn with the shape of its
  sound, a source whose file is not there is refused, and a node that
  is not a source is refused.
- **The source's payload** carries the peaks — two numbers for each
  bucket — and the measurement is what fills them.

### What it does not settle

- **One band for the whole sound.** The tree draws a band for each
  channel, one above the other, and this draws one band for the whole
  sound: the extent of every channel together. A clip's bar is one row
  here, and a row has room for one shape.
- **The shape does not follow the automation.** A track's gain over
  time changes how loud the sound is, and the shape on the bar does
  not say so — the shape is the sound's own, and the automation is
  heard (§ the automation is heard) and not seen.
- **The measurement can go stale.** The peaks are the file as it was
  when it was measured, and a file that changes afterwards is not
  re-measured until somebody measures it again.
- **The peaks are a table of the source, not of the clip.** A clip
  that fades is drawn with its fade as a notch and its shape as the
  sound's own, so the two answers are in one picture and they do not
  agree with each other about the loudness.

## 155. The resampler is a windowed sinc

§137 said that the resampler was **linear**, that a sound resampled by
a line has the softness a line has, and that a better resampler is a
filter — and that a filter is a plugin. Then it went to look for the
tree's own resampler, and the tree's resampler is **not a plugin**: it
is a windowed sinc in a library, in the tree's own dsp crate. Nobody
had gone to look.

### What was built

- A **windowed sinc** resampler in the audio crate: the sample at a
  place in the new rate is the old sound read with a low-pass filter,
  and the filter's shape is the ideal one — a sinc — with a **Kaiser**
  window over it, so that the filter's edge falls to nothing instead
  of ringing.
- The **anti-alias**: what was above the new half of the rate is taken
  away. A sound that is played at a lower rate has less room, and what
  does not fit folds back into the sound as a whistle.
- The resampler **refuses** a sound that would be longer than the
  limits allow at the new rate: it is not truncated, because a
  truncated sound is wrong in a way nobody can hear.

### What the port found

- **The tree's resampler is a library, and the port's reason not to
  have one was wrong.** §137 read the tree's resampler as a plug-in
  and concluded that a better resampler needed a device and v2 has
  none. The tree's resampler is `dsp`'s `resample`: a Kaiser-windowed
  sinc, a polyphase bank and a per-tap fallback, in a **library** that
  knows nothing about devices. The port had been told a thing about
  the tree that the tree did not say, and it had believed it — and
  the belief was in the README, where a reader would have found it.
- **A filter is not a plugin.** The port's reason was that the tree's
  plugins are better and are plugins. They are: the tree's *effects*
  are plugins. The tree's *resampler* is arithmetic in a library, and
  the same is true here: the resampler is a function in the audio
  crate, and the thing that needs it is a render, and a render is a
  command.
- **The edge of a sound shorter than the kernel is the kernel's, not
  the sound's.** A sinc kernel is wider than a few samples, so at the
  start and the end of a short sound the kernel reads outside it and
  the sound is quieter there. The tests changed when the resampler
  changed: the old ones measured the first sample of a three-sample
  ramp, which is the one place a kernel cannot be right.

### What it cost

- **Five tests** in the audio crate (`crates/craftkit-audio/tests/resample.rs`):
  a tone keeps its pitch, what was above the new half is taken away, a
  tone inside the new half is kept, a sound that would be too long is
  refused, and the channels are kept apart.
- **Two tests changed**: the ramp tests measured the edges, which are
  the kernel's now, and they measure the middle instead.
- The resampler takes the **limits** and answers with a refusal, like
  the mixer does.

### What it does not settle

- **The polyphase bank is not ported.** The tree precomputes the
  kernel for every phase of a rational rate change, so that an output
  is one dot product over contiguous memory; this resampler reads the
  kernel per tap and interpolates it. The sound is the same and the
  time is not: a long render is slower here.
- **The kernel's constants are the tree's** — thirty-two zero
  crossings, a Kaiser beta of nine, a cutoff of 0.94 when the rate
  falls and 0.97 when it rises — and they were read out of the tree
  rather than worked out here. They are the usual choices and the
  port did not measure them.
- **A stretched clip is still a varispeed** (§143): the sound is
  played more slowly and the pitch falls with it, and a resampler
  that changed the rate without changing the time would be a
  different thing.

## 156. The shape follows the automation

§154 drew the shape of a clip's sound, and said that the shape does
not follow the automation: a track's gain over time changes how loud
the sound is, and the shape on the bar did not say so.

### What was built

- The **gain** of a clip's sound at a time: the clip's own gain and
  the track's **automation** — the volume lane in decibels and the
  mute lane, which is one or less.
- The shape on the bar is drawn with that gain, so a clip that runs
  across a breakpoint is **loud at one end and quiet at the other**.

### What the port found

- **The shape is the sound as it is heard, and not the sound as it is
  on the disk.** The tree's waveform multiplies the gain in at every
  pixel, and the gain is the clip's own *and* the track's curve. The
  port had drawn the sound as it is on the disk, which is the sound
  before the mixer has had it — and the mixer is what a reader is
  looking at the timeline to understand.
- **A muted track is silent however loud its sound is.** The mute lane
  is not a gain of minus a hundred decibels: it is one or less, and
  the shape of a muted track is flat.
- **The clip's own curve is not on the bar.** A clip may carry a curve
  of its own (§147), and that curve moves the clip's sound; the shape
  on the bar follows the *track's* curve and not the clip's, because
  the clip's curve is a thing the clip draws for itself.

### What it cost

- **Three tests** in the crate (`waveform_tests`): the gain follows
  the automation, a muted track is silent, and the shape on the bar
  falls when the gain does.

### What it does not settle

- **The gain is worked out per bucket, and not per sample.** The shape
  has two hundred and fifty-six buckets and the curve is read at the
  middle of each; a curve that moves inside a bucket is a curve the
  shape cannot show.
- **The shape does not follow the pan.** A track's pan lane says where
  the sound is and the shape is in the middle of the bar, so a sound
  that is panned is drawn as if it were not.
- **The shape is the sound's own extent, and the gain scales it.** A
  sound that is six decibels down is drawn six decibels lower, which
  is what the reader wants; it is not what a meter would say.

## 157. The caret moves a word and a line at a time

§151 said that the caret moved a character at a time and that the
family's caret moves a word at a time and a line at a time.

### What was built

- The caret moves a **word** at a time: `word_right` goes to the end
  of the word the caret is in and then to the start of the next one,
  and `word_left` goes to the start of the word the caret is in and
  then to the start of the one before it.
- The caret moves a **line** at a time: `up` and `down` keep the
  caret's **place** across the line, so the caret goes to the
  character of the line above whose place is nearest its own.

### What the port found

- **A line is a thing of the layout, and the layout is not always
  there.** A paragraph with no measure has no lines, so there is
  nothing to move between, and the command says so. The port found
  this because the caret's *character* needs nothing but the payload —
  the caret can stand in a paragraph that has never been measured —
  and a movement that needs lines needs a layout that may not be
  there.
- **The word is in the words, and the line is in the drawing.** The
  word-wise movement reads the payload's words: a word is a run of
  characters with no space in it, and the spaces are the gaps. The
  line-wise movement reads the drawing: the characters are placed as
  the paragraph is drawn, and the lines are the runs of characters
  that share a baseline. Two different homes for two different
  movements, and the caret does not care which is which.
- **The caret keeps its place across a line.** The rule is not "the
  same character number on the line above" — that is the same *place*
  only if the lines are the same width — but "the character whose
  place is nearest", which is what a reader means by "up".

### What it cost

- **Five tests** in the battery (`tests/movement.rs`): the caret moves
  a word at a time, a line at a time, it keeps its place across a
  line, it stays where there is no line, and a paragraph with no
  measure has no lines to move between.
- **Two functions** in the battery and one more place of the drawing's
  made public to the crate.

### What it does not settle

- **The word is a word and not a piece of punctuation.** The text is
  split on whitespace and nothing else, so a word with a comma in it
  is a word with a comma in it, and there is no rule that says where
  the word ends and the punctuation begins.
- **The line is the line of the layout and not the line of the
  reader.** A paragraph that is set ragged has lines of different
  lengths, and the caret moves to the nearest place on the line
  above — which is the line the layout made, not the line the reader
  thinks is there.
- **There is no selection to move.** The caret moves alone; the
  selection is the session's and it is not moved with it (§152).

## 158. The resampler has a bank

§155 ported the tree's windowed sinc and said that the **polyphase
bank** was not ported: the tree precomputes the kernel for every
phase of a rational rate change, so that an output is one dot product
over the samples, and this resampler read the kernel for every tap.

### What was built

- The **bank**: the kernel precomputed for every phase of a rational
  rate change. Two whole numbers have a common factor, and the rate
  change is then a whole number of phases; each phase has its own set
  of weights, and an output is one dot product over the samples.
- The resampler uses the bank when there is one worth having, and
  reads **tap by tap** when there is not: a rate change with no small
  common factor has one phase for every output frame, and a bank of
  that size is not a bank.

### What the port found

- **The bank is the tree's, and the tree's is a library too.** The
  bank is arithmetic like the resampler it speeds up: it is a table of
  weights and a dot product, and it needs no device and no host — the
  same finding as §155, one layer down.
- **A rate change has a common factor or it has a hundred thousand
  phases.** Forty-four thousand and one hertz have no common factor
  and the phases are one for every output frame; the bank would be
  millions of weights for nothing. So the bank is **refused** when it
  is too large, and the taps are there for it — the same bargain as
  everywhere else in this tree: a bound is a bound and there is a way
  round it that is not a truncation.
- **The bank and the taps are the same sound.** That is the claim
  worth a test: if the fast path were a different resampler, the
  sound would be different in a way nobody could hear. The two agree
  to within a hundredth of a percent.

### What it cost

- **Two tests** in the crate (`bank_tests`): the bank and the taps are
  the same sound, and a rate change without a bank is read tap by tap.
- **One measurement** the reader can run: `cargo test -p craftkit-audio
  -- --ignored --nocapture` prints the two paths' times. On this
  machine, forty-four thousand and one hertz to forty-eight thousand,
  two hundred and eighteen thousand samples: **the bank took 230
  milliseconds and the taps 383** — a speed-up of **1.7 times**.

### What it does not settle

- **The bank's dot product gathers across the channels.** The samples
  are interleaved here, so the taps of a channel are a stride apart,
  and the dot product is a gather and not a walk over memory that is
  all together. The tree's samples are a channel at a time, and its
  bank is contiguous. The sound is the same and the memory is not.
- **The speed-up is not the tree's.** The tree's bank is over
  contiguous memory and its taps are the same taps, so its speed-up
  is a little better than the **1.7 times** measured here. The port
  measured what it built and not what the tree built.
- **The cap on the bank's size is a number the port chose.** Four
  million weights is sixteen megabytes, and a rate change that needs
  more than that is read tap by tap. The tree's cap is the same
  number, and neither of the two ports measured it.

## 159. The bar has a band for each channel

§154 drew the shape of a clip's sound and said that there was **one
band for the whole sound**, where the tree draws a band for each
channel, one above the other.

### What was built

- The **peaks** are the peaks of each **channel**: the table holds, for
  each bucket, the least and the most of every channel.
- The **bands**: a clip's bar is as tall as the channels it holds, and
  each channel's sound is drawn in its own band.
- The buckets and the channels **share** the table: it is a thousand
  numbers however many channels there are, so a stereo sound has half
  the buckets of a mono one.

### What the port found

- **The channels share the table, and that is a rule and not a
  detail.** The peaks live in a payload, and a payload's table is
  bounded; a sound with eight channels and two hundred and fifty-six
  buckets each would be four thousand numbers and would not be read at
  all. So the buckets are counted so that the table is the same size
  whatever the sound is — and a sound with many channels has a coarser
  shape, which is the honest bargain.
- **The band is a **band**, not a fraction of a band.** The tree
  draws a lane for each channel and the sound in it; here the bar is
  one row tall and the channels are stacked in it, so a stereo clip's
  two bands are each half a row. The shape is right and the room is
  not: a sound with eight channels in one row is eight thin bands.
- **The shape of a channel is the shape of that channel.** The port
  had been measuring the least and the most of *every* channel
  together, so a stereo sound with a tone on the left and silence on
  the right was drawn as a sound that was half as loud as it was. The
  test that found it is one line: the left channel's numbers and the
  right channel's numbers are not the same numbers.

### What it cost

- **Two tests** in the battery: the peaks are told apart by channel,
  and a stereo clip has a band for each channel.
- **Two tests changed**: the table is a thousand numbers now, and not
  five hundred and twelve, because the buckets and the channels share
  it.
- The peaks' **shape** in the payload changed — the table is per
  channel now — and a file written before this arc holds a table of
  one band and is read as a table of one channel.

### What it does not settle

- **A file written before this arc holds the old table.** The format
  writes a payload as it is and knows nothing of what is in it, so a
  sound measured before this arc is read as a mono sound with half
  the buckets it should have. The tree's format has no version for
  the payload's meaning, and this arc did not make one.
- **The band is thin.** A row is one unit tall and a sound with many
  channels has many bands in it, and the shape of each is a sliver.
- **The pan is still not on the bar.** A sound that is panned is drawn
  as if it were not (§156).

## 160. The send is panned, and it may be muted

The send's payload had a **pan** in it and the command could not set
it; the tree's send has a pan and a **mute** as well, and v2 had
neither.

### What was built

- The send's **pan**: `sound.send.add` takes it, and the copy is
  panned on its way to the bus. A pan is a number between minus one
  and one, and a number that is not one is refused.
- The send's **mute**: a muted send is a copy of **nothing** — the
  track is still heard on its own and the bus hears nothing.

### What the port found

- **The field was there and the door was not.** The send's payload
  already carried a pan — the mixer already panned the copy with it —
  and the command wrote a zero there every time. The port had read
  the mixer and found the pan working, and had not asked who could set
  it: a field that is always zero is a field nobody has looked at.
- **A muted send is a copy of nothing, and not a copy of silence.**
  The two are the same sound, and the difference is in the *rule* and
  not in the sound: a copy of silence is still a copy, and a copy of
  nothing is not a copy at all. So the mute skips the copy rather
  than making it silent — which is the same shape as the soloed track
  (§ the track is soloed), where the rule is about the other thing.
- **The pan is a **balance** and not a place.** The pan law here is
  the constant-power one: the sound in the middle is three decibels
  down, and a sound at the side is as loud as it can be. It is the
  tree's default law and it was read out of the tree.

### What it cost

- **Three tests** in the battery (`tests/send.rs`): a panned send is
  heard on its side, a muted send is a copy of nothing, and a pan that
  is not a pan is refused.
- **Two fields**: the pan was there and the mute is new — and a new
  field is additive, so a file written before this arc has no mute in
  it and its send is not muted.

### What it does not settle

- **The pre-fader send is not built.** The payload carries the flag
  and the mixer ignores it: a pre-fader send is a copy of the track's
  signal *before* the fader, and the mixer keeps one signal per track
  — so a pre-fader send is a second copy of it, and the tree does not
  keep two.
- **The pan does not follow the track's pan.** The tree's send has a
  *follow main pan* flag; here a send's pan is its own.
- **The pan is a balance between the channels and not a place in the
  room.** A sound with more than two channels is panned between the
  first and the second and the rest are left as they are.

## 161. The fader comes after the inserts, and a send may be taken before it

The tree's mixer runs a track's **inserts** and then its **fader**, and a
send may be a copy of the signal from **before** the fader. Here the
fader came before the inserts and a send was always taken after it.

This settles the open point of §160, which said the pre-fader send was
not built.

### What was built

- The **order of the track's stage**: the clips, the **inserts**, the
  **mute**, the **fader**, and then the **balance** when the track is
  routed. The fader is the track's volume and it comes after the
  inserts — the tree's order, read out of the tree.
- The **pre-fader send**: `sound.send.add` takes `pre_fader`, and the
  copy is taken from the signal before the fader. A send that does not
  say so is a copy of the signal after the fader, as it always was.
- The **mute gates the tap**: a muted track is not heard anywhere, and
  the copy a send takes is a copy of silence.
- The **automation in three steps** — the mute, the fader and the
  balance — because a send may take its copy of the signal from
  between them.

### What the port found

- **The tree's order is written down in its own words.** The header of
  the tree's mix engine reads: *clips (with clip gain and fades) →
  trim/phase → inserts → pre-fader sends → fader and mute (with
  automation) → pan → post-fader sends → busses → aux inputs → main →
  master faders*. The fader is late and the balance is later still, and
  the two sends are on either side of them.
- **The tree keeps two copies of a track's signal**, and it says so in
  the field's own name: the strip's buffer and a *pre-fader copy for
  pre-fader sends*. The second copy is made only when a send asks for
  it — a copy nobody asked for is a waste of the machine's memory.
- **The mute is two rules.** The mute *lane* gates the audio **and** the
  pre-fader tap; the mute *button* skips the send altogether, so a
  muted track's sends carry nothing at all. A rule about the copy is
  not the same as a rule about the sound, and the tree keeps them
  apart.
- **The balance belongs to the route and not to the signal.** The tree
  applies the track's pan when the track is routed, and a send's copy
  is balanced by the **send's** own pan: the track's balance does not
  travel into the send.
- **A wart of v2, found by the split.** The balance used to be part of
  one step with the fader, and a track with a volume lane and no pan
  lane lost three decibels — the middle of the constant-power law —
  while a track with no lane at all did not. The lane that moves the
  volume changed the level of a sound that was balanced. The stage is
  in steps now, and the balance only happens when there is a balance to
  apply.

### What it cost

- **Four tests** in the battery (`tests/fader.rs`): a pre-fader copy is
  taken from before the fader, a post-fader copy from after it, a
  muted track is not heard by its own pre-fader send, and an insert is
  heard by a pre-fader send.
- **A second copy of a track's signal**, held only for the tracks
  whose sends ask for it — the tree does the same.
- **The test sound is quiet on purpose.** The master is a file and a
  file cannot hold more than one, so a sum over one is clipped and the
  two copies cannot be told apart. The first attempt at this arc
  failed for exactly that reason, and the failure was in the test and
  not in the mixer.

### What it does not settle

- **The master fader is not there.** The tree runs the main mix and
  then the master faders, and v2 has no master lane and no master
  automation.
- **The tree's delay compensation is not there.** A strip whose inserts
  are slow is delayed to line up with the others, and the pre-fader
  copy has its own alignment; here every copy is aligned by
  construction, because it is the same signal.
- **The mute button is not the mute lane.** The tree has a mute on the
  strip and a mute in the automation, and they are two rules; here the
  mute is only the lane.
- **A stereo send goes straight to the sides in the tree**, and the
  send's pan is for a mono copy; here a stereo copy is panned as well.

## 162. The master has a fader of its own

The tree's mixer runs the **master faders** over the main mix, and the
metronome is summed after them. Here a master was a track like any
other, and its fader moved nothing but its own — empty — signal.

### What was built

- A track whose kind is `master` is the **fader of the whole mix**: it
  holds no clips, and its lanes move the mix.
- The **master's inserts** are run over the mix, before its fader — the
  tree's order.
- **Two masters are a stack**: each one moves what the one before it
  left, in the order of the tree.
- The **metronome is summed after the master fader**, so the click
  reaches the speakers without passing through it.
- A **clip on a master is refused by name**: a master holds a fader and
  not a playlist.

### What the port found

- **The name was already there and nobody read it.** The kinds of a
  track are open — a string in the track's payload — and `master` was
  one of the names `sound.track.add` took and wrote down in its own
  documentation. The mixer never asked a track what it was, so the
  name was accepted, stored and ignored. A name that is taken and
  never read is a name nobody has looked at, and the port had read the
  *kinds* of the tree and not the *meanings* of the names.
- **The tree's master is a strip over the mix and not a track that sums
  clips.** The main mix is copied into the master's strip, the master's
  inserts are run over that copy, and then the fader moves it. The
  master's clips are not the mix: the mix is the sum of the tracks and
  the buses, and the master is over all of it.
- **The metronome is after the fader, and it was after it by accident.**
  The tree sums the click after the master fader so that it reaches the
  speakers without passing through a fader, a track, a send or an
  insert. Here the click was already mixed in after the master sum —
  the right place for the wrong reason, and now it is the right place
  for the right reason too.
- **The tree allows several masters.** Each one is a strip over the
  mix, and they run in the order of the tree, so the last one is the
  one that is heard last. A stack of faders is not a mistake to be
  prevented; it is a thing a person may want.

### What it cost

- **Four tests** in the battery (`tests/master.rs`): a master fader
  moves the whole mix, it comes after the buses, two masters are a
  stack, and a master holds no clips.
- **No new command and no new kind**: the master is a track whose
  payload says `master`, and the open kinds are what make that
  possible. The command that made it — `sound.track.add` with
  `track_kind` — was already there.
- **The crate's own header was stale**, and is now true: it said
  *seven kinds* and *seven commands*, and there are **thirteen** and
  **twenty-three**. The header had not been read since the mixer was
  built.

### What it does not settle

- **The master's meter is not there.** The tree measures the main mix
  at the master, and v2's meters are a different thing.
- **The master has no balance.** The tree's master fader moves the
  level and not the pan, and here the master has a mute lane and a
  volume lane and no balance — which is the tree's shape.
- **An aux input is not there.** The tree's track of the kind `aux`
  reads a bus and is a strip like any other; v2's tracks hold clips,
  and nothing reads a bus back.
- **The delay compensation of the main mix is not there.** The tree
  delays the main mix by the slowest insert on the master.

## 163. A bus is read, and a reader is a track

A bus is a **destination**: sound collects there and is not heard
there. Something must read it and take it into the mix. The tree's aux
is a strip that reads a bus, and v2 had no reader at all.

### What was built

- A track may **read** a bus: the command is `sound.track.input`, and
  the reading is a **relation** — the node says what it is and the
  relation says what it reads.
- A bus that is read is **heard through the reader and not by itself**:
  the reader carries the bus's sound into the mix, with its own inserts
  and its own fader.
- The refusals that keep the reader honest: a reader **holds no
  clips**, has **no sends** and is **not routed** — it carries the
  bus's sound and nothing else, and each of those is refused by name.
- A track reads **one** bus: the same bus twice is refused, and so is
  a second bus.

### What the port found

- **The tree's bus is silent on its own.** The tree's bus is a
  destination and nothing more; what reaches the speakers is the sound
  of an aux track that reads it, at the aux's own level. Here the bus
  was heard by itself, at a level of one, and nothing could take it out
  of the mix — which is why a send was the only way to hear a bus, and
  why the copy could not be balanced or faded on its own.
- **The tree's aux is a strip like any other.** It has inserts, it has
  a fader, it has a mute, and it reads a bus. So a reader here is not a
  special kind of thing: it is a **track** that reads a bus, and
  everything a track can do to its signal — the inserts, the fader, the
  mute — it does to the bus's sound.
- **A reader is not a second path into the mixer.** It holds no clips
  (it has a bus and not a playlist), it has no sends (it carries the
  bus's sound and does not copy itself anywhere) and it is not routed
  (it is heard in the main mix). Each of those would make a loop: the
  bus's sound would be taken out of the mix, copied back into it and
  taken out of it again. The refusals are not caution; they are what
  keeps the arithmetic a sum and not a spiral.

### What it cost

- **Five tests** in the battery (`tests/reader.rs`): a bus that is read
  is heard through the reader, a reader holds no clips, a reader has no
  sends, a reader is not routed, and a track reads one bus.
- **One command** (`sound.track.input`) and **one relation kind**
  (`input`). The MCP server now carries **212** commands.

### What it does not settle

- **A reader may not be routed.** The tree's aux may be sent to a bus
  like any other strip; here a reader is heard in the main mix and
  nowhere else, because a routed reader would put the bus's sound back
  into the mix after the bus had been read.
- **The readers are not in the dependency order.** The tree reads its
  auxes in dependency order — an aux that reads a bus another aux feeds
  is read after it. Here a reader reads a bus that is finished, and a
  reader may not feed a bus, so the order is a single step; a chain of
  readers is not built.
- **The bus's channel format does not fold.** The tree's matrix folds a
  mono bus into a stereo aux with the pan law; here the copy is the
  copy.

## 164. The route says where a track is heard

The tree's strip goes to its **output** and to nowhere else: a track
that feeds a bus is heard on that bus and not in the main mix. Here the
route was a relation that the mixer drew and the render ignored, so
every track was heard in the main mix whatever it fed.

### What was built

- The **route is honoured** in the render: a track that feeds a bus is
  heard on that bus and not in the main mix.
- A bus that **feeds another bus** is heard there and not by itself,
  and the buses are fed in the order they are read in, so a bus is fed
  before it is read.
- The **order of the buses** is a helper of its own now, shared by the
  routing evaluator and the render — the evaluator worked it out and
  the render needed it too.

### What the port found

- **The route was drawn and never read.** The mixer drew the route as a
  line from the track to the bus, the evaluator listed who feeds whom
  and even worked out the order the buses are fed in — and the render
  summed every track into the main mix whatever it fed. A relation
  that two other parts of the tree use and the part that makes the
  sound does not is a relation that was never finished.
- **The tree's strip has one output.** The tree's route is a field on
  the strip and it is either the main mix or a bus, never both: a
  track that feeds a bus is not heard in the main mix, and a track that
  feeds nothing is heard there. Here a track could be fed into a bus
  and still be heard whole, which is the sound of a mixer that has not
  decided where the track goes.
- **The order of the buses was already there.** The routing evaluator
  sorts the buses so that a bus that feeds another comes before it —
  the order in which a bus must be fed before it is read. The render
  wanted exactly that order and did not have it, and the port had read
  the evaluator and not the render.

### What it cost

- **Three tests** in the battery (`tests/route.rs`): a routed track is
  heard on its bus, an unrouted track is heard in the main mix, and a
  bus that feeds a bus is heard there.
- **The render grew a step**: the tracks, then the routed tracks, then
  the buses, then the buses in order, then the readers, then the
  master. The order of the steps is the order of the sound.

### What it does not settle

- **A track may not feed a bus and be read at the same time.** The
  tree's strip may be read by an aux and feed a bus; here the two
  would be a loop, and only one of them is allowed.
- **The route does not change the level.** The tree's matrix folds a
  mono bus into a stereo bus with the pan law and the channel format
  of the destination; here the copy is the copy.
- **A cycle in the routing is not a hang and not an error.** The order
  is worked out by walking it, so a bus that is already in the order is
  left where it is; a cycle is therefore a routing that means nothing,
  and nothing says so.

## 165. The track has a trim and a phase

The tree's mixer applies the track's **trim** to its strip before the
inserts, and the **phase** is a sign and not a level: the sound is the
same and upside down. v2 had neither, and a clip's gain was the only
level a track had.

### What was built

- A track carries a **trim**, in decibels, and a **phase**, and the
  command `sound.track.set` writes them.
- The trim is the level of the track's **input**: it is applied before
  the inserts, so an insert sees the trimmed sound, and the pre-fader
  copy carries it.
- The phase **inverts** the sound and changes nothing else.
- A trim outside the tree's range — minus one hundred and forty-four
  decibels to twenty-four — is **refused by name**, and not clipped.

### What the port found

- **The tree has two gains and v2 had one.** The tree's strip has a
  *trim* at its input and a *fader* at its output, and they are
  different things: the trim is the level of what comes in, and the
  fader is the level of what goes out. A port that reads the mixer and
  sees one gain in the payload has read the fader and missed the trim.
- **The trim is before the inserts and the fader is after them.** The
  tree's chain is *clips → trim/phase → inserts → pre-fader sends →
  fader and mute → pan*, and the order is the tree's own. A trim that
  came after the inserts would be a second fader and not a trim.
- **The tree clamps the trim and v2 refuses it.** The tree's model
  clamps the value into its range when the session is written; here a
  number outside the range is refused and says so, because a refusal
  that never truncates is worth more here than a value that is quietly
  changed.

### What it cost

- **Three tests** in the battery (`tests/trim.rs`): a trim is the level
  of the input, a phase is a sign and not a level, and a trim that is
  not a trim is refused.
- **Two fields** in the track's payload, and a field is additive: a
  track written before this arc has no trim and its level is one.

### What it does not settle

- **The master has no trim.** The tree's master is a track and could
  have one; here the master's stage is its inserts and its fader and
  nothing else.
- **The trim is a number and not a lane.** The tree's trim is a field
  on the strip and not a curve, so it cannot be automated; here it is
  the same, and the tree's *fader* is the thing that has a lane.
- **The tree's input monitoring is not there**: a track that is
  monitored hears the hardware input instead of its clips.

## 166. A clip may carry effects of its own

The tree's **clip effects** are an equaliser and a compressor on one
clip and not on the track, and they run over that clip's own sound
before it is mixed into the track. v2 had no effect that was a clip's
own.

### What was built

- An insert is on **one thing**: a track or a clip, and not both and
  not neither. The command says which, and says it by name.
- A clip may carry a **chain of effects of its own**, and they run
  over the clip's sound before it is mixed into the track.
- The clip's **own gain** now comes before the clip's effects, as it
  does in the tree, where the gain is multiplied into the clip before
  the clip's chain runs.

### What the port found

- **The tree's clip effect is a pair and not a chain.** The tree's
  clip carries an equaliser, a compressor and a gain, read from the
  tree's key-value store, and the chain is a fixed pair that is either
  there or bypassed. Here a clip's effect is any plugin in the kit's
  own set — a plugin is a program and the set is open (§141) — so the
  shape is the tree's and the contents are not.
- **The clip's gain was in the wrong place.** The tree multiplies the
  clip's gain into the clip while it renders it, and the track's
  inserts run over the sum afterwards. Here the gain was applied when
  the clip was summed into the track, which is *after* the track's
  inserts: with a gain insert the two are the same, and with anything
  that is not a gain they are not. A port that reads the mixer and
  sees a gain applied at the sum has seen where the gain was *used* and
  not where the tree applies it.
- **An insert that is on nothing is an insert nobody can hear.** The
  command took a track and nothing else, so a clip's effect had no way
  to be said. The lane command had already found the shape — one thing
  or the other — and the insert command had not.

### What it cost

- **Three tests** in the battery (`tests/clipeffects.rs`): a clip may
  carry an effect, the effect is the clip's own, and an insert is on
  one thing.
- **The clip's gain moved**, and with it the order of two steps in the
  render. With a gain in an insert the sound is the same; with
  anything else it is not, and it is now the tree's order.

### What it does not settle

- **The tree's equaliser and compressor are not in the kit.** The
  kit's own set is a gain, a low pass and a delay (§ the plugins), so
  a clip's effect is one of those and not an equaliser.
- **The tree's clip effect is bypassed by a flag and not by an
  insert**: `clip_fx.bypass.<clip>` is a flag on the session and not a
  node in the document.
- **The tree's clip effects are read from a key-value store**, which
  is the tree's own shape for a parameter that belongs to no node;
  here an insert is a node and carries its parameters.

## 167. A track has takes, and one of them is played

The tree's track has several **playlists** and an index of the one
that is played. v2 had playlists and played every one of them at once:
two takes of a track were the same sound twice.

### What was built

- A track may have **takes** — `sound.playlist.add` — and one of them
  may be the one that is **played** — `sound.playlist.active`.
- The take is **named**: a name is what a person says and an index is
  what a program says.
- A track whose active take is **not there** plays its first one, so
  that a track is never silent by accident.
- A take that is not there, and a take with a name the track already
  has, are **refused by name**.

### What the port found

- **The playlists were there and the choice was not.** v2's track came
  with a playlist called `A` and the clip command could name another
  one — and the render walked every clip of the track and played them
  all. A take is a *choice*, and a choice that is not in the tree is
  not a choice: the tree's track holds several playlists and says which
  one is played, and v2 held several playlists and said all of them.
- **The tree's active take is an index and v2's is a name.** The tree
  keeps `active_playlist: usize` beside its playlists. An index is
  broken by a take being deleted — the takes move and the index does
  not — and a name is not. The port prefers the name, and says so: the
  two are the same thing said by two kinds of program.
- **A track that is silent by accident is the worst kind of silence.**
  The tree's index always points at a take that exists. Here the name
  may name a take that has gone, and the rule is that the first take
  is played instead: a track is never silent because a take was
  removed.

### What it cost

- **Three tests** in the battery (`tests/take.rs`): only the take that
  is played is heard, the first take is played by default, and a take
  that is not there is refused.
- **Two commands** (`sound.playlist.add` and `sound.playlist.active`)
  and **one field** in the track's payload — the name of the take that
  is played — which is additive: a track written before this arc plays
  its first take.

### What it does not settle

- **The tree's recording is not there.** A take in the tree is what a
  recording makes: the transport records into a take and the take
  becomes the one that is played. Here a take is made by hand.
- **The tree's takes are drawn on the track's row**, and v2's sound
  battery draws nothing at all.
- **A take is not a clip.** The tree's clips belong to a playlist and
  are moved between takes; here a clip belongs to the take it was
  added to, and there is no command that moves it.

## 168. A clip may be muted

The tree's renderer **skips** a muted clip and the tree's editor keeps
it, so that a clip that is quiet is a clip that may be loud again. v2
had no way to say that a clip is quiet.

### What was built

- The command `sound.clip.mute` says whether a clip is **muted**, and
  the render skips the clips that are muted.
- The clip stays in the document: a muted clip is a clip that is not
  played and not a clip that is gone.
- A node that is not a clip is **refused by name**.

### What the port found

- **The tree's mute is a flag and not a level.** The tree's clip has a
  `muted` field and the renderer skips the clip outright — the sound is
  not made quieter, it is not made. A port that reads the mixer and
  finds a gain on every clip has read the clip's gain and not the
  clip's mute.
- **The mute is not the clip's gain at minus infinity.** A gain of
  minus a hundred and forty-four decibels is a number, and the tree's
  mute is a decision: the clip is not read from the source at all. The
  difference is heard in the file: a muted clip that is the only clip
  renders an **empty** file, because the render's length is the end of
  the last clip that is heard.
- **The tree keeps the muted clip in the document and on the row.** A
  clip that is muted is still a clip, and it is still where it was; the
  tree draws it quiet and says nothing about it being gone. The port
  that reads a file and finds a muted clip in it has found a clip that
  was muted and not a clip that was deleted.

### What it cost

- **Three tests** in the battery (`tests/clipmute.rs`): a muted clip is
  not heard and is still there, a clip that is unmuted is heard again,
  and a node that is not a clip is refused.
- **One field** in the clip's payload and **one command**, and the
  field is additive: a clip written before this arc is not muted.

### What it does not settle

- **The tree's rating and colour are not there.** The tree's clip
  carries a rating, a colour override and a sync point, and they are
  what the tree's editor reads them for. The **sync point** is here
  now, and it is the **beat** of the clip (§196).
- **The tree's locked clips are here.** The tree's clip has an
  `edit_locked` and a `time_locked`; the kit has its own lock on a node
  (§115), which is a lock on the node and not on the clip's time. The
  clip's own two locks are the **beat**'s siblings in the battery's
  payload, and they are §197.

## 169. A path's points may be moved

The tree's path is a list of **anchors**, and each anchor carries two
handles: the one the curve leaves by and the one it arrives by. v2 had
a path that could be created and combined and **not edited**: nothing
touched a point of it.

### What was built

- The command `vector.path.move` moves a **point** of a contour, and
  the handles of the curves move with it, as they do in the tree.
- The command `vector.path.handle` sets the two handles of a point, and
  a handle on a straight segment makes it a **curve**.
- A point that is not there, and a number that is not a point at all,
  are **refused by name** — an index is a whole number and is not
  rounded into one.

### What the port found

- **The kit's path is segments and not anchors.** The tree's path is a
  list of anchors and each anchor carries its handles; v2's path is a
  list of segments, and a segment is a line or a curve with two
  control points. The anchors are therefore **derived**: the points of
  a contour are the point it starts at and the point each segment ends
  at, and a point's outgoing handle is the control point of the curve
  that leaves it and its incoming handle the control point of the curve
  that arrives. The tree's model and the kit's model say the same
  thing in two shapes, and the port had to say which shape it is.
- **A closed contour's last curve has no handles here.** The tree's
  anchors hold the handles of the curve that closes the contour; v2's
  closed contour closes with an implicit line, so the handles of that
  curve are not in the path at all. A port that reads the family and
  sees the handles of a closing curve has seen the family's model and
  not the kit's.
- **The battery could make a path and could not touch it.** The five
  path operations — unite, subtract, intersect, simplify, offset — all
  work on whole paths, and not one of them moved a point. A shape that
  cannot be edited is a shape that can only be replaced, and the tree
  is a program for editing shapes.

### What it cost

- **Four tests** in the battery (`tests/points.rs`): a point may be
  moved, a handle makes a curve, the handles move with the point, and a
  point that is not there is refused.
- **Two commands** (`vector.path.move` and `vector.path.handle`) and no
  change to the SDK: the points are worked out from the segments, and
  the kit's path is what it was.

### What it does not settle

- **A path may not be given a new point.** The tree's
  `path.insertAnchor` and `path.appendAnchor` put an anchor in and
  take one out; here a point is moved and a handle is set, and the
  number of points is the number the path was made with.
- **The tree's path is one kind and v2's is a payload.** The tree's
  path is a closed enum of what a path may be — a polygon, a
  rectangle, an ellipse, a star, a custom outline — and every one of
  them is matched in the tree's editor. Here the path is one thing and
  the shape is a property of it.
- **The tree's anchor is a point with two handles and a type.** The
  tree's anchor says what it is — a corner, a smooth point — and the
  editor draws it differently. v2's points have no kind.

## 170. A path may be painted

The tree's path carries a **fill** and a **stroke**, and two commands
say what they are. v2's path carried both and no command could write
either: the payload had a colour and a line and nothing ever put them
there.

### What was built

- The command `vector.path.fill` says what is **inside** the path, and
  the rule that says what is inside is the path's own.
- The command `vector.path.stroke` says what the **line on the
  outline** is — a colour and a width — and the line may be taken away
  again.
- A colour that is not a colour is **refused by name**, and the kit's
  spelling of a colour — `#rrggbb` — is the one every battery takes.

### What the port found

- **The payload had fields that nothing wrote.** The vector node's
  payload has carried a fill and a stroke since the battery was built,
  and the three shape commands took a fill as an argument — so a path
  made by a command had a colour and a path that came from anywhere
  else had none. The port had read the payload and not asked who fills
  it in: a field that is only written by one command is a field that
  only one shape has.
- **The tree's stroke is a colour and a width and a join and a cap**,
  and v2's stroke is a colour and a width. The join and the cap are
  the next thing, and they are in the kit's own vocabulary and not in
  the payload: a stroke is drawn by the backend and every backend
  wants the same words for it.
- **The tree's fill is not only a colour.** The tree's paint carries
  gradients as well as flat colours (§ the gradients), and a colour
  that is not a colour is refused rather than read as black: a path
  that is painted the wrong colour is a path that is wrong.

### What it cost

- **Four tests** in the battery (`tests/paint.rs`): a path may be
  filled, a path may have a line, the line may be taken away, and a
  colour that is not a colour is refused.
- **Two commands** (`vector.path.fill` and `vector.path.stroke`) and no
  change to the payload: the fields were there and are now written.

### What it does not settle

- **A stroke has no join and no cap.** The kit's stroke is a colour
  and a width, and the rasteriser draws a square at every corner
  because a square is what a machine can draw; the tree's renderer
  draws the join the document asks for.
- **A fill is a colour and not a paint server.** The tree's fill may
  be a gradient, and a gradient is a list of stops and a direction.

## 171. A path says what is inside it, and two paths may be excluded

The tree's path says **which parts of it are inside**, and it can keep
what two paths do not share. v2's path had a rule that nothing could
change, and a library that could exclude and no command that said so.

### What was built

- The command `vector.path.rule` says which parts of a path are
  inside: the default rule, or the one that does not care which way
  round a contour runs.
- The command `vector.path.exclude` keeps what two paths do **not**
  share — the difference the other way round — and the two paths
  become one.
- A rule that is not a rule is **refused by name**, and each rule has
  two spellings, so that a caller may say it either way.

### What the port found

- **The rule was in the kit and nothing could change it.** The SDK's
  path has carried a fill rule since it was written, the geometry
  library honours it, and the vector battery had no command that said
  which rule a path has. A rule that is always the same rule is not a
  rule; it is a constant with a name.
- **The library could exclude and no command said so.** The geometry
  library's boolean operation has four cases — everything in either,
  what is in both, what is in the first and not the second, and what
  is in one or the other but not both — and the battery exposed three
  of them. The fourth was in the library, tested there, and out of
  reach of a person: the port had read the library and counted the
  commands and did not put the two together.
- **The exclusion is not the difference.** The difference of two
  rectangles that overlap is one rectangle; the exclusion is two
  pieces, one on each side of the overlap. A port that has only ever
  seen the difference has seen one shape and not the rule.

### What it cost

- **Three tests** in the battery (`tests/rule.rs`): a path may say
  what is inside it, a rule that is not a rule is refused, and two
  paths may be excluded.
- **Two commands** (`vector.path.rule` and `vector.path.exclude`) and
  no change to the SDK or the library: the rule and the exclusion were
  both there.

### What it does not settle

- **The tree's rule is a property of the shape and not of the path.**
  The tree's shapes are kinds — a polygon, a rectangle, an ellipse —
  and each kind knows its own rule. Here the rule is the path's, and
  any shape may have either.
- **The tree's divide and trim are not here.** The tree's boolean
  operations include *divide* (which keeps the pieces) and *trim* (which
  cuts to the other shape); here the exclusion keeps the pieces that
  are in one and not the other, and there is no command that keeps
  every piece.

---

## 172. The PDF's filters, undone

A stream in a PDF file may be **encoded**: the `/Filter` entry names the
way, and the bytes in the file are the encoded ones. v2's PDF battery
ported the **object model** — the dictionary, the stream, the reference
— and left the bytes as bytes: a stream node named a file, and nothing
in the tree said what was inside it.

### What was built

- The module `filters` in the PDF battery holds the **five ways** a
  PDF stream may be encoded: **Flate** (a zlib stream), **LZW** with the
  early change, **ASCIIHex**, **ASCII85** and **RunLength**.
- The command `pdf.stream.decode` reads the file the stream node names,
  undoes the filter and writes what was inside it to another file.
- The **filter may be named in the command** or **in the dictionary**:
  the `/Filter` entry is where a PDF file says which filter a stream is
  in, and the command falls back on it.
- More than one filter may be named, and they are undone in the
  **reverse** order: the last one named is the first one undone.
- A filter the kit does not know is **refused by name**, and a stream
  that names no filter at all is refused too.
- The bytes are **bounded on both sides**: what comes in and what comes
  out, and a stream that would grow past the bound is refused before the
  memory is spent.

### What the port found

- **The kit already had the inflate.** A Flate stream is a zlib stream,
  and the PNG crate's inflate is the kit's zlib reader — so the PDF's
  most common filter was one function call away. That is a finding about
  the *kit*, not about the PDF: a second inflate would have been a
  second answer to a question the tree had already answered.
- **The filters could not be a crate of their own.** A crate may only
  depend on a **strictly lower** layer, and the inflate lives in the
  PNG crate, which is a **bytes-only** library at the bottom. A filter
  crate would have to sit below the PDF battery, and it could not reach
  the PNG crate, because that is the same layer. So the filters are a
  **module inside the PDF battery**: the one place where the rule lets
  them live.
- **The early change is the whole difference in LZW.** The PDF's LZW
  grows the code width one entry *before* the table is full, and a
  decoder that does it the other way — the way a GIF's does — reads the
  wrong bytes and says nothing about it. The port had to write the
  decoder against the specification and not against the other one.
- **A decoded stream is not the stream.** The node still names the
  **encoded** bytes, because that is what the PDF file holds; the
  decoded bytes are what the caller asked for, and they go to a file of
  their own. Rewriting the node would have been a second path from a
  command that only reads.
- **The five are decoders and not encoders, in the tree as well.** The
  tree reads PDF streams and does not write them, and a filter that
  undoes a stream is worth having on its own: the encoder is a different
  job with different questions — what the file will look like after —
  and this tree does not answer those.

### What it cost

- **Seven tests** in the battery (`tests/filters.rs`): the five filters,
  the dictionary naming the filter, and the refusals.
- **One command** (`pdf.stream.decode`), **one module** (`filters`) and
  **one dependency** (`craftkit-png`). The MCP server now carries
  **213** commands.

### What it does not settle

- **The predictors are not here.** A predictor is a second step on top
  of Flate or LZW — the PNG and TIFF predictors — and the `/DecodeParms`
  entry names it. It is the next thing this battery needs, and it is
  not this arc.
- **There is no encoder.** A file may be read and undone, and nothing
  may be written back.
- **The bounds are the kit's, not the caller's.** A stream may hold 256
  mebibytes and may decode to 64; a caller that knows the file it wants
  cannot yet say so.
- **The stream is read whole.** The bytes are read into memory before
  they are undone, so a stream that is large and cheap to decode is
  still large in the heap.

---

## 173. The PDF's predictors, undone

A filter undoes the **encoding** of a stream's bytes. A **predictor**
undoes a step that was made after the encoding: the bytes of an image
are stored as the difference from the byte before them, so that the
bytes of a picture compress far better than the picture does. The PDF
names the predictor in `/DecodeParms`, and v2's battery could not undo
one.

### What was built

- The module `predictors` holds the **two** predictors a PDF uses: the
  **TIFF** one — the difference from the byte to the left — and the
  **PNG** ones — each row of the picture begins with a byte that says
  how the row was made, and there are five ways.
- The command `pdf.stream.decode` undoes the predictor **after** the
  filter, which is the order the steps were made in.
- The predictor may be named in the **command** or in the
  **dictionary**, and the **parameters** — the columns, the colours and
  the bits to a component — come from the dictionary either way.
- A predictor the kit does not know is **refused by name**, and so is a
  row that is shorter than the picture is wide.

### What the port found

- **The predictor is not a filter, and it is not another crate
  either.** It is a step on top of a filter, and the battery that
  holds the filters holds it: a separate crate would sit at the same
  layer and could not reach the filters it works on.
- **The width of the picture is the dictionary's, and the command
  cannot supply it.** The PNG predictors work a **row** at a time, and
  a row is as wide as the picture is — which the PDF says in
  `/Columns`. So the command may say *which* predictor a stream is in,
  and the *width* comes from the file: a caller that says "predictor
  15" and nothing else has said nothing about how wide the picture is,
  and the kit's default is the PDF's own — one column.
- **The Paeth guess is a guess about three bytes.** The fifth PNG
  predictor does not store the difference from the byte to the left or
  from the byte above; it stores the difference from the **nearest of
  the three**, and the nearest is the one the guess names. A decoder
  that implements the other four and this one wrongly reads rows that
  are one byte out and does not say so.
- **The five PNG ways are the PNG's, not the PDF's.** The PDF points at
  the PNG's predictors and reuses the numbers; the rows and the guess
  are the PNG specification's, and the port had to write them against
  that and not against the PDF's words about them.

### What it cost

- **Six tests** in the battery (`tests/predictors.rs`): the two
  predictors, the dictionary naming one, the Paeth guess, the
  predictor that is none, and the refusals.
- **One module** (`predictors`), **one parameter** on the command that
  already existed, and no new command: the MCP server still carries
  **213** commands.

### What it does not settle

- **There is no encoder.** A predictor that undoes a step is worth
  having on its own; the tree's PDF reads and does not write.
- **The bits are not split.** A picture of sixteen bits to a component
  is two bytes to a component, and the kit rounds the byte up and does
  not unpack the halves; the difference the predictors undo is made on
  the byte, not on the bit.
- **The parameters are the PDF's defaults.** One column, one colour and
  eight bits are what the PDF says when it says nothing, and a caller
  that wants a picture of another shape must say so in the dictionary.

---

## 174. A line's corners and ends

A line that turns has a **corner** in it, and a line that stops has an
**end**. The corner is pointed, round or cut off; the end is flush, round
or square. v2's stroke carried a colour and a width and nothing else, so
every line in the tree had the same corner and the same end — and the
rasteriser drew a **square** at every point of the outline, which is
none of the six.

### What was built

- The SDK's **stroke** carries a **join** and a **cap**: `miter`,
  `round` and `bevel` for the corner, `butt`, `round` and `square` for
  the end.
- The rasteriser draws them: the joint fills the gap on the **outside**
  of the turn, and the **miter** is the point where the two edges meet
  — a miter longer than four widths is a bevel, because a long spike of
  ink is not what a corner means.
- Two commands: `vector.path.join` and `vector.path.cap`. A name that
  is not one of the three is **refused by name**, and a path with no
  line has no corners and no ends to set.

### What the port found

- **The seam was the SDK's, and that is where it belongs.** The
  rasteriser is **below** the batteries and cannot ask a battery what
  its lines should look like; so the join and the cap are in the
  **stroke**, beside the colour and the width, and every backend that
  draws a stroke is told the same thing.
- **The tree's own drawing was a joint of its own.** The rasteriser put
  a square at every point of the outline — a corner that is neither a
  miter nor a bevel nor a round, and an end that is neither flush nor
  round nor square. It filled the gaps, which is what it was for, and
  it did it the way a machine can: in squares.
- **The default had to be chosen, and the tree's pictures moved.** The
  default is the **miter** and the **flush** end — the way a PostScript
  line has always been drawn — and the square the rasteriser used to
  put at the end of a line is gone: an open line now stops at its last
  point. That is a visible change to the tree's own drawings, and it is
  a change the arc meant to make.
- **The join and the cap are of the line and not of the path.** A path
  has a **fill** with a rule, and it has a **line** with a corner and
  an end; the two are different things and the payload keeps them
  apart.
- **The file grew a field and no version.** A stroke written before the
  join and the cap existed is read with them: the two are **optional**
  in the file, and a stroke that says nothing about them means the
  default. That is why the format did not need a new version for it.

### What it cost

- **Five tests**: three in the vector battery (`tests/stroke.rs`) and
  two in the rasteriser's own (`tests/picture.rs`).
- **Two commands** (`vector.path.join` and `vector.path.cap`). The MCP
  server now carries **215** commands.
- **The SDK's stroke grew two fields**, and the eighteen places in the
  tree that make a stroke were taught to say what they mean.

### What it does not settle

- **There is no anti-aliasing.** A round corner is a polygon of twelve
  sides and a round end is a half circle of twelve sides, and the
  pixels are on or off (§67).
- **The miter limit is the kit's.** Four widths, and not the caller's.
- **The join and the cap are only of the drawing.** The geometry
  library's boolean operations do not know them — they are about what
  is inside a path and not about the ink on its outline — and the tree
  has no outline-of-a-stroke operation either.

---

## 175. A page tree, and what a page inherits

A PDF's pages are a **tree**, not a list: the root is a page tree node,
and the pages hang under it. And a page may leave out what a page does
not have to say — the **media box**, the **resources**, the **crop box**
and the **rotation** come from the nearest ancestor that has them.
v2's PDF battery had the object model and nothing that said which
object was a page.

### What was built

- The command `pdf.page.add` puts a **page** in the document: an
  indirect object whose dictionary says `/Type /Page`, with a
  **parent** if it has one.
- The tree is **two edges**, as the file has it: the page names its
  parent under `Parent` and the parent names the page under `Kids`.
- The command `pdf.page.attributes` says what a page **has**, and
  where each thing came from: the four inheritable keys are found by
  walking up the tree, and the nearest ancestor that has one wins.
- A page whose type is not `Page` is **refused**, and a page with no
  parent inherits nothing.

### What the port found

- **The tree was already in the tree.** The battery had a relation of
  the kind `ref` whose payload says which **key** the reference is
  under — and `Parent` and `Kids` are keys. So the page tree is not a
  new kind of edge: it is two references with the right keys, and the
  model says so.
- **The inheritance is a question, not a value.** A page's media box is
  not a number the page holds; it is the number the page *has*, which
  is its own when it says one and its nearest ancestor's when it does
  not. That is why the command is a query and the document is not
  changed: the answer depends on the tree around the page, and writing
  it into the page would be a second copy of something the tree already
  says.
- **The nearest ancestor wins, and the tree may be deep.** The walk is
  bounded — a page may not have more than sixty-four ancestors — because
  a document that says otherwise is not walked for ever.
- **A page that says what it wants keeps it.** The inheritance is for
  what a page does not say, and a page that says a media box of its own
  says it, whatever its parent says.

### What it cost

- **Four tests** in the battery (`tests/pages.rs`): a page may be put
  in the document, a page inherits what it does not say, a page that
  says keeps what it says, and a page with no parent inherits nothing.
- **Two commands** (`pdf.page.add` and `pdf.page.attributes`). The MCP
  server now carries **217** commands.

### What it does not settle

- **The page tree is not read from a file.** The battery still has no
  parser and no cross-reference table, so the tree is something a
  person or a program says; a file's own tree is not yet in the
  document.
- **The inherited keys are the four of the specification.** A key that
  the specification does not inherit is not inherited here either, and
  a page that wants one must say it.
- **The tree is not walked downwards.** The command answers for one
  page; there is no command that lists the pages under a node in the
  order the tree has them.

---

## 176. A page says what it draws

A page's **content** is a stream of operands and operators: `10 20 m`
moves the pen to a point, `100 100 200 200 re` draws a rectangle and
`f` fills what the pen has drawn. The PDF's content is how a page is
drawn, and v2's PDF battery could not read a byte of it.

### What was built

- The module `content` **reads** a content stream: it says what the
  stream says, in the order the stream says it.
- The command `pdf.content.operators` reads a stream — decoded first,
  if it is encoded — and reports every **operator** with the
  **operands** it was given.
- The operands are the PDF's own: the numbers, the **names** (`/F1`),
  the **strings** (`(text)` and `<hex>`), the **arrays** and the
  **dictionaries**.
- An operator the kit does not know is **refused by name**, and so is
  a stream that ends with operands and no operator.

### What the port found

- **The bytes of a content stream are usually not encoded.** The
  decode path demands a filter — a stream that is being decoded must
  say what it is in — and the content path does not: a page's content
  is the plain bytes unless the file says otherwise. Two commands, one
  reader, and the difference is one question: *does this stream say
  what it is in?*
- **A string is a sequence of bytes and not text.** The kit does not
  pretend otherwise: each byte of a PDF string is one character of the
  answer, so nothing is lost and nothing is guessed. A PDF's text is
  in its fonts, and a font is not yet in this tree.
- **An inline image is not a list of operands.** Between `BI` and `EI`
  the bytes are the image, and a reader that read them as operands
  would find a picture full of operators. The operator is reported and
  the bytes are skipped — and that is a gap, not a solution: the image
  itself is not read.
- **The operator is a word and the operand is its value.** The PDF
  writes them the other way round — the operands first and the operator
  last — and a reader that took the first word of a pair as the
  operator would read every stream backwards.

### What it cost

- **Three tests** in the battery (`tests/content.rs`): the operators
  and their operands, the names and the strings and the arrays, and
  the refusals.
- **One command** (`pdf.content.operators`). The MCP server now
  carries **218** commands.

### What it does not settle

- **The operators are not carried out.** The module says that a `re` is
  there; what a `re` draws is the evaluator's business, and the
  evaluator does not draw a PDF page yet.
- **The graphics state is not followed.** `q` and `Q` save and restore
  the state, `cm` changes the matrix and `gs` brings in an extended
  state; the command reports them and does not track them.
- **The inline images are not read**, and the text is not laid out.

---

## 177. A path is drawn once and placed somewhere

A path is drawn once and **placed** somewhere: the six numbers of a
matrix say where. v2's vector battery had a path and a colour and a
line, and no way to say where the path was — so a path could only ever
be where it was drawn.

### What was built

- The geometry library holds a **matrix**: the six numbers `a b c d e f`
  of an affine transform, with the point `(x, y)` moved to
  `(a·x + c·y + e, b·x + d·y + f)`.
- The command `vector.path.transform` places a path, and the matrix
  stays in the payload.
- The path in the payload is the path **as it was drawn**, and the
  transform says where that path is: the path is **drawn**, **hit** and
  **measured** where the transform puts it, and its box moves with it.
- A matrix that is not six numbers is **refused by name**, and so is
  one that collapses the plane: a path that cannot be undone cannot be
  hit.

### What the port found

- **The matrix belongs to the geometry library and not to the
  battery.** A matrix is an operation on a shape, and the library is
  where the operations on a shape live; the battery owns the *command*
  and the *payload*, and the library owns the arithmetic. The library
  is a library and not a plugin: it adds no command and the linker
  does not wire it in.
- **The path in the payload is not the path in the world.** A path is
  drawn once and placed, and a payload that held the placed path would
  lose the drawing: the path could not be moved, scaled or rotated
  again, because the transform that did it would be gone. So the
  payload holds the path and the transform, and the world sees the two
  together.
- **The hit test has to go backwards.** The point a person clicks is
  in the **world** and the path is in its own place, so the point is
  moved back into the path's space before it is tested. That is why a
  matrix that cannot be undone is refused: there would be no way to
  ask where the person clicked.
- **A path written before the transform was in the payload has none,
  and the file did not need a new version.** The transform is
  **optional** in the file, and a path that says nothing about it is
  where it is — the same way the stroke's join and cap are optional.

### What it cost

- **Three tests** in the battery (`tests/transform.rs`): a path may be
  placed, a placed path is where the transform says, and a transform
  that cannot be used is refused.
- **One command** (`vector.path.transform`). The MCP server now
  carries **219** commands.

### What it does not settle

- **The transform is not in the shape.** The SDK's path is the shape a
  file carries and a canvas draws, and it has no matrix of its own; a
  program that wants a transformed path outside this battery must
  transform it itself.
- **The stroke is not transformed.** The line is drawn by the canvas
  with the width it is given, and a path that is scaled is not a path
  with a wider line: the line keeps its width.
- **There is no gradient and no clip.** The canvas has no way to say
  either, and a transform is the last of the three that a path needs.

---

## 178. The objects a file keeps in one place

A PDF file is made of **objects**, and each one is a value: a number, a
name, a string, an array, a dictionary, a boolean, null, or a
**reference** to another object. And since PDF 1.5 the objects may be
kept in an **object stream** — a stream that holds them, so that they
may be compressed together. v2's PDF battery could read a page and its
content, and not one of the objects underneath.

### What was built

- The module `objects` reads the **values** of a file: the numbers, the
  names, the strings (both spellings), the arrays, the dictionaries,
  the booleans, null, and the references.
- The command `pdf.objstm.read` reads the **object stream**: the
  dictionary says how many objects it holds — `/N` — and where the
  first one's data begins — `/First` — and the **header** says where
  each object is.
- A stream that does not say what it holds is **refused**, and so is
  one whose header does not hold the objects it says.

### What the port found

- **The object stream's two numbers are the whole of its shape.** The
  count and the first offset are in the dictionary, and the header is
  that many pairs of integers — the number of an object and the offset
  of its data, counted from `/First`. So an object stream is a stream
  like any other: the plugin already knows how to read a stream and
  undo its filter, and the object stream is what is left over.
- **A dictionary is a list of pairs, and it is so here too.** The
  plugin's own dictionaries are lists of pairs because a JSON object
  does not keep the order of its keys; the objects inside an object
  stream are read the same way, so a dictionary that came out of a
  stream and a dictionary that was typed in are the same thing.
- **A reference is not a value.** It is two numbers and the letter `R`,
  and it says which object is meant — so it is reported as a reference
  and not as a number, and finding the object it names is the file's
  business and not this module's.
- **The strings are bytes.** The same rule as the content stream's:
  each byte is one character, so nothing is lost and nothing is
  guessed, and a file's text is in its fonts.

### What it cost

- **Three tests** in the battery (`tests/objstm.rs`): the objects of an
  object stream, the values of an object, and the refusals.
- **One command** (`pdf.objstm.read`). The MCP server now carries
  **220** commands.

### What it does not settle

- **There is no writer.** A file may be read and not written, and an
  object stream may be read and not made.
- **The references are not resolved.** The reader says which object a
  reference names; it does not go and find it.
- **The file is still not read.** There is no parser and no
  cross-reference table, so the objects are read out of a stream that
  somebody has already found.

---

## 179. The file's own index of itself

A PDF file does not hold its objects in order. It holds them wherever
they were written, and a **cross-reference table** says where each one
is. The table is found from the end of the file, which says where the
table is. v2's PDF battery could read a page, a page's content and an
object stream — and it could not find the file's own index.

### What was built

- The module `xref` reads the **table** of a file, and there are two
  kinds of it:
  - the **classic table**, a list of twenty-byte entries under the word
    `xref`, and a **trailer** after them;
  - the **cross-reference stream**, the same information in binary,
    which is how a file compresses its own index.
- The command `pdf.file.xref` reads a file and reports the **entries**
  and the **trailer**.
- An entry says which of the three it is: in the file, **free**, or
  **compressed** — in an object stream.
- A file that does not say where its table is is **refused**.

### What the port found

- **The table is found from the end, and the end is a small place.**
  The last thing a file says is where its table is, and the table may
  be a stream of its own — so the reader follows the file's own
  instruction instead of looking for objects.
- **The two tables are the same table.** The classic one has entries of
  twenty bytes with the numbers written out; the stream one has the
  same three fields — the kind, the place and the generation — in
  binary, each as wide as the dictionary says. So the reader answers
  the same question either way: where is object *n*?
- **A number in a file is not always a whole number, and the kit's
  JSON is not either.** The values that come out of a file's own
  dictionary are numbers, and the kit's JSON holds them as numbers that
  may have a fraction; so the reader asks for the number and rounds it
  down where a count is wanted, and a table that says something else
  is refused by name.
- **The file is still not read.** The table says where an object is;
  fetching it, and following the references, is the next thing.

### What it cost

- **Three tests** in the battery (`tests/xref.rs`): the classic table,
  the cross-reference stream, and the file that says nothing.
- **One command** (`pdf.file.xref`). The MCP server now carries
  **221** commands.

### What it does not settle

- **The objects are not fetched.** The table is an index and the reader
  reads the index.
- **The incremental save is not followed.** A file that has been saved
  again has more than one table, and the newest one wins — here the
  reader takes the one the end of the file points at, and the older
  ones are not read.
- **The hybrid files are not read.** A file may have both kinds of
  table, and the reader takes the one it is given.

---

## 180. A file read into the document

A PDF file is an **index** and the objects it points at. The index is
read, the objects are read, and the document holds them: an object is a
**node** and a reference is a **relation**. v2's PDF battery could read
a file's pieces one at a time, and could not hold a file.

### What was built

- The command `pdf.file.load` reads a file and puts it in the
  document: every object becomes a node of the kind `pdf.object` or
  `pdf.stream`, and every reference becomes a relation of the kind
  `ref`.
- The bytes of a stream go into a **file of their own**, beside the
  document, because the bytes are not the document.
- The graph is built in **two passes** — the objects first, and the
  references after — so that a reference which points **forward** is
  still a relation.
- The keys and the names are written the **kit's way**: without the
  slash in front, so that a loaded dictionary says the same thing as
  one that was typed in.

### What the port found

- **The bytes of a stream cannot live in the payload.** The kit's rule
  is that the bytes travel beside the document in the bundle, so the
  loader writes them out and the node says which file they are in. The
  alternative — the bytes in the payload — would have made every
  stream a document the size of a picture.
- **A reference that points forward is why there are two passes.** The
  kit's rule is that a relation names two nodes that are *in* the
  document, so every object has to be there before any reference is
  made. The file does not care: an object may refer to one that is
  written later.
- **The dictionary's spelling had to be settled.** A file writes its
  keys as `/Type` and its names as `/Catalog`; the kit's payload writes
  them as `Type` and `Catalog`. The loader translates, so that a
  loaded object and a typed one are the same object — and the price is
  that a name and a string look alike, which the plugin's header has
  said from the start.

### What it cost

- **Two tests** in the battery (`tests/loader.rs`): a file is loaded
  into the document, and the references become relations.
- **One command** (`pdf.file.load`). The MCP server now carries
  **222** commands.

### What it does not settle

- **There is no writer.** A file may be read into the document; the
  document may not be written back as a file.
- **The incremental save is not followed.** A file that has been saved
  again has more than one table, and the loader takes the one the end
  of the file points at.
- **The page tree of a loaded file is a tree of relations, not a tree
  of nodes.** The relations say which page names which parent, and the
  command `pdf.page.attributes` walks them — but the pages of a loaded
  file are ordinary objects until somebody says they are pages.

---

## 181. The mix, in the order its routes say

The tree reads its **auxes** in dependency order: an aux that reads a
bus another aux feeds is read after it. v2's mixer read the buses and
then the readers, and a reader could not feed a bus at all — so the
chain of auxes the tree has was not built.

### What was built

- A reader may now be **routed**: it may feed a bus, and it is heard
  there and not in the main mix.
- The mixer works out the **order of the mix**: the buses and the
  readers, mixed so that a bus is mixed before the readers that read
  it, and a reader is mixed before the bus it feeds.
- A mix that **loops** — a reader that feeds the bus it reads — is
  **refused by name** when the sound is rendered.

### What the port found

- **The order was a property of the code and not of the document.** The
  mixer mixed the buses, then the readers, and the two steps were
  written down in that order; a chain of readers had nowhere to go,
  because the reader's sound was always the main mix's.
- **A reader that feeds a bus is not in the mix.** That was the
  reason the refusal was there: a routed reader would have put the
  bus's sound back into the mix after the bus had been read. It is not
  a reason to refuse — it is a reason to *order* the mix — and the
  order is what the routes say.
- **The tree's rule about derived values held.** The order of the mix
  is not stored anywhere: it is worked out from the routes every time
  the sound is rendered, and a document that says otherwise gets a
  different order — which is the point of a document.

### What it cost

- **Two tests** in the battery (`tests/mixorder.rs`): the chain is
  heard at its end, and a mix that loops is refused.
- **One test changed** in the battery's own file (`tests/reader.rs`):
  a reader that was refused a route may now have one, because the
  refusal was the thing this arc removes.
- **No new command**: the MCP server still carries **222** commands.

### What it does not settle

- **The bus's channel format does not fold.** A mono bus is not
  folded into a stereo one; that is still the tree's own law and not
  this tree's.
- **The order is worked out on every render.** The mixer sorts the
  stages each time the sound is rendered, and a document with many
  buses pays for it each time; the order could be kept, and nothing
  keeps it.
- **A reader may still not have sends.** It reads one bus and it is
  heard there or in the mix; a copy of it to a third bus is still
  refused.

---

## 182. The shape a line covers

A line with a width covers an **area**, and that area is a shape in its
own right: the path of the line, widened and closed at its ends. The
tree has that shape as an **effect** — the outline of a stroke — and
v2's geometry library could not make it, and no command asked for it.

### What was built

- The geometry library holds `outline_stroke`: the shape a line of a
  given width covers, with the line's **join** and **cap**, as a path.
- The command `vector.path.outline` makes that shape a **path of its
  own**: a new node, filled, that may be measured and cut like any
  other. The path it came from is left alone.
- A path with **no line** has no outline, and the refusal says so.

### What the port found

- **The stroke was already carrying what the outline needs.** The
  join and the cap — §174 — are the two things that say what the shape
  of a line looks like at its corners and at its ends; so the outline
  is a shape with the same corners and the same ends as the line it
  came from, and the arc that gave the line its join and its cap gave
  the outline its shape for free.
- **The width of a line is the width of the outline and not its
  radius.** A line of width four covers two on each side of the path,
  and an outline that used the width as a radius would be twice the
  size.
- **The outline is a band and not a shape inside the path.** The
  outline of a square is the ring around it, and not the square: the
  outline of a closed path has a hole in it, which the fill rule
  decides how to read.

### What it cost

- **Two tests** in the battery (`tests/outline.rs`): a line's outline
  is a path, and a path with no line has no outline.
- **One command** (`vector.path.outline`). The MCP server now carries
  **223** commands.

### What it does not settle

- **The outline is not cleaned.** The tree's outline is one clean
  shape — the overlaps removed and the pieces fitted together — and
  here the outline is what the stroker gives: a path whose pieces may
  overlap where the line crosses itself.
- **The miter limit is the kit's.** Four widths, as §174 said.
- **The outline is not in the file format.** It is a path, so it is —
  but there is no "outline" of a path in the tree's own format, and
  the outline here is a path like any other.

---

## 183. The document, written out as a file

The reader reads a file into the document; the **writer** writes the
document out as a file. The two are the same questions asked in the
other order: where each object is, and what it says. v2's PDF battery
could read a file and could not write one.

### What was built

- The module `writer` writes the **objects** — each with its number and
  the word `obj` — the **cross-reference table** that says where they
  are, and the **trailer**, which says how many there are and which
  one is the root.
- The command `pdf.file.write` writes the document out: every object
  of the battery becomes an object of the file, and a **stream** takes
  its bytes from the file they live in.
- The root is the object that was named, or the one whose type is
  `Catalog`; a document with neither has no root, and the refusal says
  so.
- A file that was read in and written out is **read back** — the same
  objects, and the same references between them.

### What the port found

- **A name and a text had to be told apart.** The payload holds a
  **name** as a plain string — the kit's spelling, and the one every
  other command uses — and a **text** string as `{"string": …}`, so
  that the writer knows that `/Title` is a name and `(Hello)` is a
  text. §180 said the two look alike; this is the price of that, paid
  in the loader.
- **The length of a stream is in its dictionary, and the writer puts
  it there.** The file's own dictionary says how long the stream is,
  and the writer writes the number it has — so the length is never
  guessed and never wrong.
- **The table is a list of subsections.** The object numbers of a
  document are not necessarily consecutive, and the table says so: a
  subsection is a first object and a count, and the numbers that are
  not next to each other are in subsections of their own.

### What it cost

- **Three tests** in the battery (`tests/writer.rs`): a file is written
  out again, the file that is written is read back, and a document
  with no root is refused.
- **One command** (`pdf.file.write`). The MCP server now carries
  **224** commands.

### What it does not settle

- **The file that is written is a plain file.** No encryption, no
  object stream, and the classic table — a file that a reader must
  understand is a file that every reader understands.
- **The streams are written as they are.** A stream whose bytes are
  compressed in the document is written out compressed, and the
  dictionary says so — but the writer does not compress anything
  itself.
- **The document's own format is not the file's.** A v2 document is a
  tree of nodes and relations; a PDF file is a list of objects and a
  table. The loader and the writer are the two ends of the gap, and
  the gap is still there in the middle.

---

## 184. A point whose two handles are one

A **smooth point** is a point whose two handles are collinear and of the
same length: the curve passes through it as one curve and not as two.
The tree has the operation — *convert to smooth* — and v2's path had
handles and no command that made them agree.

### What was built

- The command `vector.path.smooth` makes a point smooth: the two
  handles take the average of the two they were, and a side that was a
  straight line becomes a curve with a handle.
- A point with **no handles** has nothing to smooth, and a point with a
  curve on only one side is refused — both by name.
- The **closing edge** of a closed contour is made a segment of its
  own when a handle needs it: an implicit edge has nowhere to put one.

### What the port found

- **A closed contour's closing edge may be implicit, and a handle has
  nowhere to go.** The SDK's polygon leaves the closing edge out — the
  contour is closed, so the edge is there — and the two commands that
  work on handles were looking at the **last segment** for the first
  point's arriving side, which is the wrong segment: it ends at the
  last point, not at the first. The port found it because the smooth
  command asked the same question and got a different answer.
- **The handles of a point are the two sides of it, and a straight
  side has none.** The tangent of a straight line is the line itself,
  so the offset of its handle is zero — and a point with a curve on
  one side and a straight line on the other is smoothed by *mirroring*
  the curve's handle, not by averaging it with nothing.
- **A straight line that becomes a curve keeps its direction at the
  far end.** The other handle is a third of the way along the line, so
  the curve leaves that end in the direction the line went — the same
  trick a path's own straight segments use when they are drawn.

### What it cost

- **Two tests** in the battery (`tests/smooth.rs`): a point with a
  curve on one side may be made smooth, and a point that cannot be is
  refused.
- **One command** (`vector.path.smooth`). The MCP server now carries
  **225** commands.
- **Two helpers fixed** in the battery: which segment leaves a point
  and which arrives at it, which took the wrong segment for the first
  point of a closed contour.

### What it does not settle

- **A point cannot be made a corner again.** The tree converts a point
  back to a corner, which frees its two handles; here the handles are
  the shape, and freeing them means putting them back where they were
  — which nobody kept.
- **The average is the average.** The tree's smooth point keeps the
  handles' directions and their lengths in a way that is not simply
  their average; here the two handles become the average, which keeps
  the point where it was and makes the curve as smooth as it can be.
- **The first point of a closed contour gains a segment.** The contour
  is the same shape, and the segment is the edge that was there.

---

## 185. The clips, butted against each other

The tree's editor lays the pieces of a take out by hand, and the
**spacing** command does it: the clips are moved so that each one
starts where the last one ended. v2's timeline could trim a clip and
stretch it and mute it, and could not move a row of clips at once.

### What was built

- The command `sound.clip.space` spaces the clips it is given: each
  one starts where the last one ended, with a **gap** between them.
- The spacing may be **start to start** as well — the starts are the
  same distance apart and the lengths are their own.
- One clip is not **spaced**, and a gap that is not a gap is refused
  by name.

### What the port found

- **The gap is in samples and not in seconds.** The tree's editor
  works in seconds, because a person edits in seconds; this tree's
  timeline is in the session's samples (§19), and a gap of a quarter
  of a second is a number of samples that depends on the tempo of the
  session — so the gap here is a number of samples and the caller
  says which.
- **Moving a clip does not move what it reads.** The clip's start is
  where it sits, and its offset is where it reads from in the source;
  spacing the clips moves the first and leaves the second — the same
  fact the trim command found when it moved the offset by the delta
  divided by the stretch.

### What it cost

- **Three tests** in the battery (`tests/space.rs`): the clips are
  butted together, they may be spaced start to start, and a spacing
  that cannot be done is refused.
- **One command** (`sound.clip.space`). The MCP server now carries
  **226** commands.

### What it does not settle

- **The locked clips are not skipped.** The tree's editor leaves a
  time-locked or an edit-locked clip where it is; this tree has no
  locks, so every clip that is named is moved.
- **The clips are spaced in the order they start.** The tree's editor
  spaces the selected clips, and so does this one, but the selection
  here is a list of ids and not the editor's selection — which is the
  session's state and not the document's.
- **The gap is not musical.** The tree's editor can space the clips
  on the beats; here the gap is a number of samples, and the caller
  works out what a beat is.

## 186. The copies follow the clip

The tree's editor loops a clip by pasting copies of it onto the track,
and it remembers the loop in a field on the clip. v2's timeline could
trim a clip and stretch it and mute it, and could not loop one — and
the copies it could not make were the copies it could not find again.

### What was built

- The command `sound.clip.loop` makes the copies of a clip: each one
  starts where the last one ended, and the last one is cut short to
  fit.
- Three ways to say how long the loop is: a **count** — the clip and
  its loops — a **length**, which is how long the whole looped clip is,
  and a **fill**, which is up to the next clip on the track or else to
  the end of the timeline.
- The command `sound.clip.unloop` takes the copies away, or leaves them
  and says that they are not a loop any more.
- The loop is a **relation**: a copy is named by the clip it follows,
  and the chain is a walk.

### What the port found

- **The tree's loop is invisible.** The tree remembers it in a field on
  the clip, and to find the copies again it looks at the track: a clip
  that starts where this one ends, of the same material. The tree even
  has a test that a loop is still recognised after the source's content
  is edited — which is the tree working around its own blindness. Here
  the copy is named by the clip it follows, and nothing is inferred.
- **A loop is a chain and not a tree.** Two clips of the same material
  butted together by hand are, in the tree, indistinguishable from a
  loop. Here the edge says which is which — and a clip that is already
  followed by a copy may not be looped again, because the chain is a
  line and a branch would have two answers to the same question.
- **The last copy is cut short, and the fades with it.** A loop of a
  whole number of bars ends exactly where it began; one that is not
  whole has a last copy that is short. The tree shortens that copy's
  fades to fit, and so does this one: a fade past the end of a clip is
  a fade that is not heard.
- **A bound is a refusal and not a shorter loop.** A clip loops at
  most a thousand times, and a loop that asks for more is refused by
  name — a loop that is not the loop that was asked for is a loop that
  is wrong in a way nobody can hear.

### What it cost

- **Six tests** in the battery (`tests/loop.rs`): the copies are made
  and they are linked, the last one is cut short, the loop fills to the
  next clip, the copies are taken away, the copies may stay, and a loop
  that cannot be made is refused.
- **Two commands** (`sound.clip.loop` and `sound.clip.unloop`). The MCP
  server now carries **228** commands.
- **One private bound**: a clip loops at most **1000** times.

### What it does not settle

- **The tree's paste overwrites.** The tree's loop pastes the copies
  onto the track, and the paste replaces whatever was under them. This
  tree has no such rule: a copy is laid beside what is there.
- **A clip that overlaps is not a neighbour.** The tree's fill stops at
  the next clip that starts at or after this one ends, and this one
  does the same — but a clip that overlaps this one and starts earlier
  is not a neighbour in either tree, and here that is a decision made
  without saying so.
- **The loop is not in the drawing.** The copies are bars of their own
  on the timeline, as they are in the tree; nothing draws them as one
  thing, because the drawing of a loop is a question for the tree.
- **The copies do not know where they came from.** A copy is a clip in
  its own right, and it says nothing about the clip it was made from,
  except that the edge says who follows whom.

## 187. The clip snaps to its neighbour

The tree's editor has an *Edit ▸ Snap to* menu with two items in it:
**Next** puts a clip's end against the start of the clip that follows
it, and **Previous** puts a clip's start against the end of the clip
that comes before it. v2's timeline could space the clips and could
not put one against another.

### What was built

- The command `sound.clip.snap` moves the clips it is given: `next`
  puts each clip's end against the start of the clip that follows it,
  and `previous` puts each clip's start against the end of the clip
  that comes before it.
- The clip **nearest the destination** is moved first, so that the
  neighbours stay where they are.
- The clips that are named are **destinations too**, and a direction
  that is not one of the two is refused by name.

### What the port found

- **The tree's editor snaps the neighbours first, and says why.** Its
  own comment says that ignoring the selected neighbours made *Snap to
  Next* pile every selected clip onto the same later clip, and made
  *Snap to Previous* leave the gap between the selected clips in
  place. Here the named clips are the destinations too, and the order
  is the tree's: the one nearest the destination goes first.
- **The tree's move clamps at the beginning.** A clip that is snapped
  back past the start of the timeline is put at the start of it, and
  its end does not meet its neighbour's end. Here the same clamp
  happens, and it is a decision the tree makes silently.
- **A snap does not overwrite.** The tree's editor moves a clip by
  taking it off the track and putting it back, and putting it back
  clears whatever was under it. Here a clip is moved and the clips
  around it are left alone — the same decision §185 made for the
  spacing, and the same one §186 made for the loop's copies.

### What it cost

- **Five tests** in the battery (`tests/snap.rs`): the clip meets the
  next one, the clip meets the previous one, the named clips are the
  destinations too, the neighbours are the clips of the same take, and
  a snap that cannot be done is refused.
- **One command** (`sound.clip.snap`). The MCP server now carries
  **229** commands.

### What it does not settle

- **The direction is a word and not a menu.** The tree's editor has two
  commands, one for each direction, and a host may put them in a menu
  side by side. Here there is one command and a parameter, because one
  tool that says which way it is going is better than two tools that
  are the same tool.
- **A clip in another take is not a neighbour.** The tree's editor
  looks at the clips of the track's **active** list, because a track in
  the tree has one list of clips at a time. Here a track may have
  several playlists — the takes — and a clip's neighbours are the clips
  of the list the clip is in.
- **The grid is not here.** The tree's editor also snaps to a grid,
  which is a **view** setting — a number of pixels a thing lands on.
  This tree's timeline has no grid, so there is nothing to snap to but
  the clips.

## 188. The annotation is an object on the page

The tree's PDF editor draws annotations on its pages: links, notes,
highlights, and the rest of the kinds the PDF names. v2's PDF battery
could read a file and write one, and had nothing to say about the
things a page carries besides its content.

### What was built

- The command `pdf.annot.add` puts an annotation on a page: its kind,
  its rectangle, its text, and — for a link — where it goes.
- The command `pdf.page.annotations` reads the annotations of a page,
  and each one says where it goes.
- The page names its annotations in the `/Annots`, and that naming is an
  **edge** — the same edge the page tree is made of.

### What the port found

- **The tree's kind is a list, and this one's is a name.** The tree's
  editor has a list of the annotations it draws and keeps the others as
  they are, and its `LinkAction` is a closed list of three — a page, a
  web address, and the things it does not edit. Here nothing matches
  on the kind: the annotation says what it is, and the **action** is an
  object of its own, so that a third party may make one. The tree's
  third case — the thing it cannot edit — is the case that needs no
  case at all.
- **The tree's rectangle is put the right way round.** The corners may
  come in any order and the tree sorts them, and a link that is less
  than two points across is refused: a link that cannot be clicked is
  not a link. The bound is the tree's own.
- **A web address is 7-bit ASCII**, and the PDF says so (§12.6.4.8).
  An address that is not is refused here too.
- **The action is written inside the annotation in the tree, and it is
  an object here.** The tree's `/A` is a dictionary inside the
  annotation's own dictionary; here it is an object the annotation
  refers to — which is legal PDF, and which keeps **one** spelling for
  a dictionary: a list of pairs.

### What it cost

- **Five tests** in the battery (`tests/annot.rs`): the annotation is an
  object on the page, a link goes to another page, a link goes to a web
  address, the annotations of a page are read, and an annotation that
  cannot be made is refused.
- **Two commands** (`pdf.annot.add` and `pdf.page.annotations`). The MCP
  server now carries **231** commands.

### What it does not settle

- **The annotation is not drawn.** v2 has no PDF renderer: the battery
  reads and writes the file, and the drawing of a page is a question
  for the tree.
- **The style is not written.** The tree's link has a style — a border
  width, a colour, a dash, an underline and a highlight. Here those
  are the annotation's own `/Border`, `/C`, `/BS` and `/H` entries, and
  nothing writes them: the dictionary holds them and the battery does
  not match on them.
- **A dictionary inside a dictionary does not survive a round trip.**
  The reader reads one as a list of pairs, and the writer writes a list
  of pairs as an **array** — so a file with a nested dictionary in it
  is written out wrong. The action of a link is an object of its own
  here, which is why this arc does not hit it; but the bug is there,
  and it is the next thing to fix.

## 189. A dictionary is a dictionary

The tree's PDF editor writes a file out of a document and reads it back,
and the two are the same question asked in the other order. v2's PDF
battery could read a file and write one — and a file with a dictionary
inside a dictionary came out **wrong**: the dictionary was written as an
array, and nothing said so.

### What was built

- The reader holds a dictionary that is inside another dictionary as a
  **list of pairs under the name `entries`** — the name the payload
  already uses for the dictionary of an object, so that there is one
  spelling for a dictionary and not two.
- The writer writes it back as a **dictionary**: the file that comes out
  of a document that was read from a file has the dictionary in it, and
  not an array.

### What the port found

- **A JSON array is one thing, and a PDF has two.** A PDF's dictionary
  and a PDF's array are both lists of things, and the payload had one
  list. The tree has two types — a dictionary and an array — and it
  never has to guess which one it is holding. Here the payload has to
  say, and it says it with the name it already uses.
- **The order is why the dictionary is a list.** A JSON object does not
  keep the order of its keys, so the dictionary is a list of pairs, and
  that is why the spelling had to grow a name rather than be the object
  itself: a JSON object would have lost the order the file had.
- **The bug was in the writer, and it was silent.** The reader read a
  nested dictionary as a list of pairs and the writer wrote a list of
  pairs as an array, so the file that came out was malformed and the
  command said it had written a file. §188 named the bug by looking for
  it; this is the fixing of it.

### What it cost

- **Two tests** in the battery (`tests/dictionary.rs`): a dictionary is
  a list of pairs and an array is an array, and the dictionary survives
  the round trip.
- **No commands**: the MCP server still carries **231** commands.
- **One spelling changed**: a dictionary inside a dictionary is now
  `{"entries": […]}`, and the values of an object stream say so too.

### What it does not settle

- **The price is paid on both sides.** A PDF dictionary with a key
  called `entries` whose value is a list of pairs is read as a
  dictionary and written as one — the same price §180 paid for a name
  and a text.
- **The tree's type is still not here.** The tree has a dictionary and
  an array and never guesses; the payload guesses and marks. A typed
  value would not have to mark anything, and that is what the kernel is
  for (§16).
- **The writer writes what the payload says.** A document built by hand
  may hold a nested dictionary in the old spelling, and the writer will
  write an array for it: the reader is forgiving and the writer is not.

## 190. The fill is a paint

The tree's vector editor fills a path with a colour or with a
**gradient**: the colours, the stops they are at, and where the gradient
is put. v2's kit had an appearance with a colour in it and nothing else
— the fill was a `Colour` in the SDK, in the canvas, and in every
plugin that drew anything.

### What was built

- The **fill** is a paint: one colour, or a gradient. The appearance
  holds one, the canvas is given one, and the vector battery's payload
  holds one.
- The **gradient** is the kit's own: the kind, the stops, and the
  geometry — a line from a point to a point, or a centre and a radius.
  The kit says what a gradient is, and it says how the stops are mixed.
- The command `vector.path.gradient` fills a path with one. Where the
  gradient is put is the path's **own box**, unless the caller says.
- The raster draws it: every pixel is asked where it is along the
  gradient, and the answer is its colour.

### What the port found

- **The tree's gradient is a type and this tree's is a value.** The tree
  has a `Gradient` with a kind, a method and a dither, and it decides
  the **geometry** in a separate place — the tree has a `GradientGeom`
  and a `GradientPaint`, and the paint is what a shape refers to. Here
  the gradient holds its own geometry, because the gradient is the kit's
  and the shape may be given one.
- **The tree's action was a closed list of three, and this one's is
  not a list at all** — the same finding as §188, seen from the other
  side: the tree's `LinkAction::Other` exists because the tree cannot
  edit an action it does not know. A gradient here is not a kind at all
  that the kit matches on; it is a list of stops, and the stops are
  whatever the caller sent.
- **The appearance could not be copied.** The appearance was `Copy`, and
  a gradient is a list of stops — so the appearance is `Clone` now, and
  the places that copied one copy it. The appearance is still the one
  place a colour lives (§95); it is now the one place a **paint** lives.
- **The file did not change.** A fill that is one colour is written as a
  colour — the same bytes as before — and a gradient is written under
  the name `gradient`. A file written before the gradient is still a
  file, and the tree's own format needs no migration.

### What it cost

- **Four tests** in the raster (`tests/gradient.rs`): the rectangle
  changes across, the path changes across, the radial gradient changes
  outwards, and the colours are mixed between the stops.
- **Two tests** in the vector battery (`tests/paint.rs`): a path may be
  filled with a gradient, and a gradient that cannot be made is refused.
- **One command** (`vector.path.gradient`). The MCP server now carries
  **232** commands.
- **The seam moved**: the appearance's fill is a paint, and the canvas
  takes one — so every plugin that fills something was touched.

### What it does not settle

- **The colours are mixed in RGB.** The tree mixes its stops in a
  **colour space** it names, and it has more than one. Here the stops
  are mixed in the bytes the colour has, which is what a picture of
  bytes can do; a colour space is a question for the colour crate.
- **The gradient is not in the tree's own drawing.** v2 has one drawing
  — the raster's — and the gradient is drawn by it. The tree has a
  swatch, a gradient tool, a mesh and a dither, and none of them are
  here.
- **The stops are not edited.** The tree's editor drags a stop along
  the gradient and gives it a colour; here the stops are what the
  command is given, and nothing moves them afterwards.

## 191. The thing mixes with what is under it

The tree's vector editor has a **transparency panel**: every object
says how it mixes with what is under it — multiply, screen, overlay,
and the rest. v2's kit had an appearance with a colour in it and
nothing that said how the colour met the one below.

### What was built

- The **blend mode** is the kit's own: the sixteen modes of the PDF and
  the screen, and the kit says how each of them mixes two colours.
- The appearance holds one, the canvas is given one, and the vector
  battery's payload holds one.
- The command `vector.path.blend` says how a path mixes, and the
  core's `core.appearance.set` says how a **shared** appearance mixes.
- The raster mixes the two colours the way the mode says, and the
  source's own opacity is laid over the result.

### What the port found

- **The tree's mode is a colour-space question, and this one's is
  not.** The tree's `BlendMode` carries a **method** — Classic or
  Perceptual — and its colours are mixed in the colour's own model:
  CMYK stays CMYK, and a Classic gradient's neighbouring stop colours
  are mixed in the model they are in rather than in display RGB. Here
  the colours are mixed in the bytes the colour has, which is what a
  picture of bytes can do — the same answer §190 gave for the
  gradient's stops.
- **The tree's mode is on the object, and this one's is on the
  appearance** — the same place a colour lives, so that two paths may
  be given the same appearance and mix the same way.
- **The tree dithers a gradient and this tree does not.** The tree
  adds fine noise of up to a colour level to a gradient's fill, which
  hides the steps between the levels an 8-bit colour has. Here the
  raster has no noise at all, and a gradient's steps are steps.
- **The file did not change.** An appearance with no mode in it is the
  plain one, and the modes are named as the PDF names them — so a file
  written before the mode is still a file.

### What it cost

- **Five tests** in the raster (`tests/blend.rs`): the modes mix the
  two colours as the PDF says, a mode is named however it is written,
  a shape mixes with what is under it, a transparent shape lets the
  backdrop through, and the mode is the same everywhere.
- **One test** in the vector battery (`tests/paint.rs`): a path may mix
  with what is under it, and a mode that is not one of the sixteen is
  refused.
- **One command** (`vector.path.blend`). The MCP server now carries
  **233** commands.
- **The seam moved again**: the canvas takes the mode, and every plugin
  that draws something was touched.

### What it does not settle

- **The colours are mixed in RGB**, and the tree's CMYK and its
  perceptual method are not here.
- **There is no dither.** A gradient's steps show as steps, and the
  tree's answer to that — noise — is not in this tree.
- **The letters do not mix.** The canvas's text takes a colour and not
  a mode, because a mode is a property of a *thing* and the canvas's
  letters are drawn one at a time.

## 192. The copies of a shape

The tree's vector editor has a **repeat**: a grid of copies of a shape,
a row of mirrored copies, a ring of copies round a centre. v2's vector
battery could make a shape and could not make another one out of it.

### What was built

- The geometry library grew the matrices that put one shape beside
  another: a **move**, a **turn about a centre**, and a **mirror over
  a line**.
- The command `vector.path.grid` makes the copies of a path in a grid;
  `vector.path.mirror` makes a mirrored copy; `vector.path.radial`
  makes a ring of copies.
- The copies are **paths of their own** — the same shape, the same
  paint, and the transform that says where each one is.

### What the port found

- **The tree's repeat is a node with the copies in it.** The tree's
  repeat is a thing that may be edited as one, and it has two
  endings: *release* turns the copies into ordinary objects, and
  *expand* turns the repeat into a group of its instances. Here the
  copies are paths of their own from the start — the state the tree
  reaches by releasing.
- **The tree's mirror repeat is a row at an angle, and not a mirror
  over a line.** Its **angle** and its **offset** say how the row is
  laid out. Here the command is a mirror over a **line**: the shape
  turned over it, once. The tree's row is a thing this tree does not
  have — and a row of mirrored copies is a grid of copies with a
  reflection in it.
- **The tree's bounds are two kinds of thing.** The tree's radial
  repeat is **refused** outside one to a thousand, and the grid's rows
  and columns are **clamped** into one to five hundred — a clamp is a
  truncation, and a grid of three hundred rows asked for is a grid of
  three hundred rows. Here the bound is one kind of thing: a shape
  that asks for more copies than the bound allows is refused by name,
  and the tree's thousand is this tree's thousand.

### What it cost

- **Four tests** in the battery (`tests/copies.rs`): the grid makes its
  copies, the mirror makes a mirrored copy, the radial makes a ring,
  and the copies that cannot be made are refused.
- **Three commands** (`vector.path.grid`, `vector.path.mirror` and
  `vector.path.radial`). The MCP server now carries **236** commands.
- **One private bound**: a shape is given at most **1000** copies.

### What it does not settle

- **The copies are not a group.** The tree's repeat may be moved and
  turned as one thing; here each copy is a path, and a group of them
  is a layer (§44).
- **The tree's grid has a default and this tree's does not.** The
  tree's grid repeat, when the spacing is not said, spaces the copies
  a quarter of the shape's size apart. Here the step is the caller's,
  because a step that is not said is a step that is not there.
- **The mirror is one shape and not a row.** The tree's mirror repeat
  makes a row of mirrored copies along an angle; here the command
  makes the one copy that is over the line.

## 193. The order of the drawing

The tree's sound editor has a **Bring to Front** and a **Send to
Back**, and its vector editor draws its shapes in the order they are
in. v2's kit said from the start that *the order of the tree is the
order of the drawing* — and no battery had a command that said it.

### What was built

- The command `vector.path.order` puts a path in front of the others or
  behind them, and `sound.clip.order` does the same for a clip.
- The kit's own `bring_to_front` and `send_to_back` now reach **the
  whole tree**: a node that is under a node that is under a node is
  moved among **its** brothers.

### What the port found

- **The kit already had the order.** The SDK's `bring_to_front` and
  `send_to_back` were there from the start, and their own words say
  what they are for: *the order of the tree is the order of the
  drawing*, and the picture and the picking agree because both walk
  the tree in that order. The gap was not the kit — it was the **door**:
  nothing in any battery reached the kit's own method.
- **The kit's order reached one level down.** The lookup was in the
  roots and then in their children, and a clip under a playlist under a
  track is three levels down: the sound battery's clips could not be
  moved at all, and the command said the node was not in the document.
  The lookup now walks the tree, and a node is moved among its brothers
  wherever it is.
- **The tree has a bug of the same shape.** Its own tests carry a bug
  called *bring to front across layers underflows*: bringing a node to
  the front across two layers underflows, and an underflow is not a
  number. This tree's layers are **edges**, so there is no list of
  layers to underflow — but the tree's layers hold an ordered list of
  their own, and this tree's do not: the order here is the order of the
  **tree**, the containment, so a node moved to the front moves among
  its brothers and not among the nodes of its layer.

### What it cost

- **Three tests** in the vector battery (`tests/order.rs`): a path may
  be brought in front, a path may be sent behind, and an order that
  cannot be given is refused.
- **One test** in the sound battery (`tests/space.rs`): a clip may be
  put in front of the others.
- **One test** in the kit (`tests/document.rs`): a node may be moved
  wherever it is.
- **Two commands** (`vector.path.order` and `sound.clip.order`). The
  MCP server now carries **238** commands.

### What it does not settle

- **The order is the tree's and not the layer's.** The tree's layer
  holds an ordered list, and a node may be brought to the front of
  **its layer**; here the layer is an edge and the order is the
  containment's, so the order of the nodes on a layer is the order of
  the document.
- **The sound battery's order is what a person sees.** The clips of a
  track are mixed and not laid over one another, so their order is the
  order they are drawn in and not the order they are heard in.

## 194. The edge is in the dictionary

The PDF battery writes files: the document goes out as a PDF file, and the
file is written from the **dictionaries** of its objects. But a reference
was only an **edge** in the document — a relation of the kind `ref` — and
an edge is not written out. So a page tree built here was a tree the
file did not have: the page said nothing about its parent, the parent
said nothing about its children, and nothing said how many children
there were.

### What was built

- `pdf.page.add` now says the tree in the dictionary too: the page
  names its parent in `/Parent`, the parent names its children in
  `/Kids`, and the parent says how many there are in `/Count`.
- `pdf.ref.add` puts the reference in the dictionary as well as in the
  document. A new parameter, `list`, says whether the key holds a **list**
  of references that grows — a `/Kids` — or one reference that is set — a
  `/Parent`.

### What the port found

- **A reference is two things at once.** It is an edge, because the
  document is a graph and a relation is the only way an object may name
  another; and it is a **value in a dictionary**, because the file is
  written from the dictionaries and not from the edges. The port had
  only the first, and the two do not agree: a graph with no dictionaries
  is a graph, and a PDF file is a set of dictionaries.
- **The count of the children is derived.** The page tree's `/Count` is
  not a number anybody keeps: it is how many children there are. The
  port counts the edges that carry the key `Kids` and writes the number
  down — the same rule the kit has said since the beginning: a value
  that can be worked out is worked out, and never kept.
- **A list key and a single key are different keys.** The `/Kids` of a
  page tree grows, and the `/Parent` of a page is set. The dictionary
  does not say which is which — the *key* says it, by what the file
  means by it — so the command asks.

### What it cost

- **Five tests** in the PDF battery (`tests/refs.rs`): a reference is in
  the dictionary, a list key grows and a single key is set, the page tree
  says itself in the dictionary, the page tree is in the file that is
  written out, and a reference to an object that is not there is refused.
- **One parameter** (`list` on `pdf.ref.add`) and one behaviour (the
  page tree in the dictionary). The MCP server now carries **238**
  commands — the same number, because no command was added: the command
  was there, and it did not say this.

### What it does not settle

- **The dictionary is written, and the edges are kept.** Both are there
  now, and they are two spellings of the same thing. A file that is
  written and then read back gives the two apart: the loader reads the
  references out of the dictionaries, and it does not look at the edges
  the document already had. Which of the two is the truth is a question
  this tree does not answer.
- **Only the page tree is derived.** `/Count` is counted because the
  page tree is the one place the port knows the rule for. A `/Length`
  of a stream is written from the bytes, a `/Size` of the file is
  written from the number of objects, and a `/N` of anything else is
  still a number somebody has to keep.

## 195. The outline of the document

The tree's PDF editor has **bookmarks**: a tree of items, each one with
a title and a place to go, and the tree's own library edits them by
**path** — `[0, 2]` is the third child of the first top-level
bookmark. v2's PDF battery had the object model and nothing that said
what a document's outline was.

### What was built

- The command `pdf.outline.add` puts a **bookmark** in the outline: a
  title, and where it goes — a page of this document. It hangs under
  the outline's root, or under another bookmark.
- The command `pdf.outline.read` reads the outline back: the items in
  the order they are in, with their titles, their destinations and
  their children.
- The outline is made when the document has none: the **catalog**
  names it, and an outline without a catalog has nowhere to hang.

### What the port found

- **The tree's outline is a list, and the tree's address is a path.**
  The file keeps the siblings of an item in a **chain** — each one
  names the next, and the last names none — so the tree's library
  walks that chain and addresses a bookmark by the chain of indices
  that reaches it. Here the chain is in the **dictionary**, as the file
  has it (`/First`, `/Last`, `/Next`, `/Prev`), and the address is the
  **node**: a bookmark is a node, and a node has an id. The path was
  the tree's way of naming a place in a chain, and this tree's nodes
  already have names.
- **The count is derived, and the sign says whether the item is
  open.** The tree's library recomputes every `/Count` after every
  edit, and the sign of the number is the state: a **positive** count
  is an item that is open and shows its children, and a **negative**
  one is an item that is closed and shows nothing. So the count here
  is worked out from the edges — how many items are **visible** below
  an item — and never kept: the root's count is how many items are
  visible at all, and an item's is how many are visible below it.
- **The count of a file's object is not always an integer.** The
  numbers in a file's bytes are read as **floats**, because that is
  what the bytes are, and a reference in a loaded file therefore holds
  `3.0` and not `3`. A reader that asks for a whole number and gets a
  float back finds nothing: this arc's first version could not read
  back a file it had written, and the fix is a reader that takes a
  number however it is spelled.

### What it cost

- **Seven tests** in the PDF battery (`tests/outline.rs`): a bookmark
  may be put in the outline, the outline is read in order, a bookmark
  may go under another, the list of siblings is relinked, the edges
  are there as well as the values, a bookmark that cannot be is
  refused, and the outline survives the round trip through a file.
- **Two commands** (`pdf.outline.add` and `pdf.outline.read`). The MCP
  server now carries **240** commands.
- **One module** (`outline`) and one node kind more in the file's
  terms: an outline item is an **object** of its own, with a `/Title`
  and a `/Dest`, and it is not a new kind.

### What it does not settle

- **The outline is not moved.** A bookmark may be added and read, and
  that is all. The tree's library moves bookmarks and renames them and
  deletes them, and each of those is a **relinking** of the affected
  siblings — the same relinking the tree's library does, and the next
  arc.
- **An item is only closed because its count is negative.** Nothing
  here makes an item closed: the sign is the state, and there is no
  command that changes it. A tree that can say *open* and *closed* is
  the next thing to port.
- **The count is the visible count and not the total.** An item that
  is closed shows nothing below it, so the count of its parent does
  not count it — which is the rule of the file, and is the rule this
  follows.

## 196. The beat of a clip

The tree's sound editor has a **sync point**: a place inside a clip
where the beat falls, and the thing a clip is lined up by. A clip
recorded against another one still lines up because both of them say
where their beat is. v2's sound battery had the tempo map, the meter
and the clips — and nothing that said where the beat of a clip fell.

### What was built

- The command `sound.clip.sync.set` puts the **beat** in a clip: a
  place on the timeline, which must be inside the clip. Without a
  place, the beat goes where the selection starts.
- The command `sound.clip.sync.remove` takes it away.
- The command `sound.clip.sync.read` reads it back: the offset in the
  clip's own samples, the place it falls on the timeline, and the
  **bar and beat** it falls on — worked out from the tempo map, and
  not kept.
- The **trim** carries the beat: the trim takes samples off the head
  of the clip and leaves the rest where it was, so the beat moves
  nearer the head by the number of samples that were cut — and a beat
  the trim left outside the clip is gone.
- The **loop** carries it too: a copy cut short loses a beat that is
  past the cut.

### What the port found

- **The tree's zero is two things.** The tree keeps the sync point as
  a number, and **zero means none** — so a clip whose beat is at its
  very first sample cannot say so, and the tree's own editor draws the
  sync point only when it is `> 0`. Here the beat is an **option**:
  no beat, or a beat at some sample in the clip. A beat of zero is a
  beat at the clip's head, and it is not the same as no beat at all.
- **The tree clamps and this tree refuses.** The tree's editor takes
  the place the beat falls at and **clamps** it into the clip, so a
  beat asked for outside the clip lands at its edge without a word.
  Here the place outside the clip is refused: a beat that was moved
  without being asked for is a beat the caller did not want.
- **The beat is a place in the audio, and the audio does not move.**
  The tree moves its sync point by hand on every edit — a trim of the
  head takes the delta off it, and a trim of the tail drops it when it
  falls outside. Here the same rule is written once, in the trim and
  in the loop, and the beat is an **offset in the clip's own samples**:
  it is where a person sees it, on the clip's bar.
- **A clip's samples and a source's samples are not the same
  distance apart.** A stretched clip plays its source more slowly, so
  a source sample and a timeline sample are not the same length. The
  beat is in the **timeline's** units, because a beat a person cannot
  place on the clip's bar is not a beat they can use.

### What it cost

- **Eight tests** in the sound battery (`tests/beat.rs`): a beat may
  be set, a beat at the head is not no beat, a beat outside the clip
  is refused, a beat follows the audio of a trim, a beat the trim left
  outside is gone, a copy cut short loses a beat past the cut, a beat
  is read with the place it falls in the bar, and a clip with no beat
  says so.
- **Three commands** (`sound.clip.sync.set`,
  `sound.clip.sync.remove` and `sound.clip.sync.read`). The MCP
  server now carries **243** commands.
- **One module** (`beat`) and one field in the clip's payload: the
  beat, an option of samples. The tree's own kinds did not change.

### What it does not settle

- **There is no grid.** The tree's editor snaps a clip to a grid, and
  the grid is a setting of its own — a division of the bar. This
  battery has a beat and a tempo map and no grid, so nothing here
  snaps a clip to anything, and the tree's `clip.quantize_to_grid`
  has nowhere to land.
- **The beat is not shown.** A sound document is not a picture here,
  so the beat is a fact in the document and not a mark on a bar. The
  tree draws it as a flag over the clip, and that flag is a question
  for the tree.
- **The clip's own metadata is still not here.** The tree's clip also
  carries a **rating**, a **colour**, an **edit lock** and a **time
  lock**, and none of those are in this tree yet — they are fields on
  the clip rather than behaviour, and they are the next slice.

## 197. The locks on a clip

The tree's sound editor has two locks on a clip, and they are two
different claims: the **edit lock** says the sound inside the clip is
finished, and the **time lock** says the clip stays where it is. v2's
sound battery had neither, and every command that changed a clip
changed a clip that was supposed to be finished.

### What was built

- The command `sound.clip.lock.edit` locks a clip's **content** — its
  sound is finished, and it is not trimmed, not faded and not
  stretched.
- The command `sound.clip.lock.time` locks a clip's **place** — it
  stays where it is, and it is not spaced and not snapped.
- A lock that is not named is **toggled**, and the two locks are two
  locks: a clip may carry both, and taking one off leaves the other.
- The commands that would break a lock **refuse** it, and say which
  lock and which clip: the trim, the fade and the stretch ask the
  edit lock, and the spacing and the snap ask the place lock.

### What the port found

- **The tree's two locks are two claims, and the kit's one lock is
  another.** The tree's clip carries `edit_locked` and `time_locked`,
  and they say different things: one is about the sound and one is
  about the place. The kit has a lock of its own (§115) — `node.lock`
  locks a **node**, so that it cannot be moved, resized or deleted —
  and it is a lock on the node and not on the clip's time. A sound
  clip has no place at all (§ the sound kinds have none), so the kit's
  lock cannot say what the tree's locks say. The clip's locks are the
  battery's own.
- **The tree's locks are checked in a few places and not in others.**
  The tree's editor refuses to trim an edit-locked clip and leaves
  locked clips out of a move, but a clip's own flags are the only
  record of the lock: nothing in the tree's **document** says what a
  lock forbids, so a reader of a tree's file cannot know which edits
  were forbidden. Here the locks are in the clip's **payload** and the
  commands ask them, so the rule is in one place.
- **A locked clip does not move, whichever lock it has.** The tree's
  move leaves a locked clip alone — and the tree's editor moves a
  locked clip out of the way when the clip it is up against is moved.
  Here the lock is asked by the command that would break it: the
  spacing and the snap refuse the whole move when one of the clips
  named is locked, because a spacing that left one clip behind would
  be a spacing the caller did not ask for. The tree's rule and the
  kit's rule are the same rule — §115 says a locked node refuses the
  whole of a selection move — and here the clip's lock refuses it too.
- **A locked clip is still a destination.** A clip may be snapped
  *against* a locked clip: the locked clip stays where it is and the
  other one moves to meet it. A lock is a claim about the clip that
  carries it, and not about the clips around it.

### What it cost

- **Seven tests** in the sound battery (`tests/locks.rs`): the two
  locks are two locks, a lock that is not named is toggled, a locked
  sound is finished, a locked place stays where it is, a locked sound
  does not move either, a locked clip is a destination, and an
  unlocked clip is changed as ever.
- **Two commands** (`sound.clip.lock.edit` and
  `sound.clip.lock.time`) and **two fields** in the clip's payload.
  The MCP server now carries **245** commands.
- **Five commands changed** — the trim, the fade, the stretch, the
  spacing and the snap — because a lock that nothing asks is a flag
  and not a lock.

### What it does not settle

- **The kit's own lock still does not reach the clip.** `node.lock`
  locks a node so that it cannot be moved, resized or deleted, and
  the sound battery's commands do not ask it: a clip whose **node** is
  locked may still be trimmed, because the lock on the node is about
  the node and not about the sound. The two locks are separate, and
  this arc did not join them.
- **The locks are not shown.** A sound document is not a picture here,
  so a locked clip is a fact in the document and not a padlock on a
  bar. The tree draws the lock on the clip.
- **The clip's marks are still not here.** The tree's clip also
  carries a **rating** and a **colour**, and neither is in this tree
  yet — they are fields on the clip rather than behaviour, and they
  are the next slice.

## 198. The marks of a clip

The tree's sound editor puts two marks on a clip: a **rating** of stars,
and a **colour** — a tint for the clip's bar on the timeline. The v2
sound battery had neither, and the last arc named the rating as the
next slice. This arc adds it, and it finds that the other mark was
already here.

### What was built

- The command `sound.clip.rating` sets the clip's **rating**: stars, from
  none to five.
- The rating is a field in the clip's **payload**, beside the beat and
  the locks, because those are the clip's own facts too — and the clip's
  kind says it in the plan.
- A rating outside the range is **refused**, and the clip keeps the
  rating it had. A rating that is not a whole number of stars is
  refused as well.
- **Zero is a rating** — the clip has none — and not the absence of one,
  so every clip carries a rating.
- The clip's **colour** is the kit's appearance and not a field of the
  clip: `core.appearance.set` sets one on any node, a sound clip
  included, so the battery has no command of its own for it.

### What the port found

- **The tree clamps a rating and its keyboard refuses one.** The tree's
  `clip.rating` takes any number and **clamps** it into the range: nine
  stars become five, and minus one star becomes none. Its keyboard does
  not pass the nine key on at all — the tree's own test asserts that a
  press of nine leaves the rating at four. So the tree has two answers
  to one question, and which one you get depends on how you asked.
  Here there is one answer, and it is the **refusal** (§ the kit's own
  rule: a refusal never truncates). A rating asked for as nine stars is
  not a rating, and a rating asked for as two and a half stars is not a
  rating either.
- **The tree's two marks are not two marks.** The rating is the clip's
  own; the colour is the **kit's**. The tree's sound editor has a
  command of its own for the colour of a clip, but the kit already says
  how every node is drawn — an **appearance** is the colour a node is
  filled with, and one command sets it on any node — so a command of the
  battery's own would be **two paths to one thing**: the tree's way for
  a clip and the kit's way for everything else. Here the clip's colour
  is the appearance the clip carries, and the tree's `clip.color` is
  `core.appearance.set` with a narrower name.
- **A rating is a mark and not a behaviour.** Nothing in the tree
  changes because of a rating — it is a number the editor's stars are
  drawn from — and here it is the same: a field in the payload, said in
  the plan, and nothing else asks it. That is the difference from the
  beat, which the audio moves when the clip is trimmed, and from the
  locks, which the commands that would break one of them ask.
- **The tree's type is the gate, and here the file is the gate.** The
  tree's rating is a `u8`, so the tree's document cannot hold a rating
  of nine stars or of two and a half: the type says no before anything
  else is asked. Here the payload is JSON, so a rating read from a file
  is a **number** — any number — and the reader refuses the ones that
  are not ratings (§ the same lesson as the PDF's numbers: a number
  read from a file's bytes is a float, and a file that says nine stars
  is a file the battery refuses and not a file it believes).

### What it cost

- **Seven tests** in the sound battery (`tests/marks.rs`): a clip is
  rated, no stars is a rating, a rating outside the range is refused,
  half a star is not a rating, the rating is in the plan, the colour of
  a clip is its appearance, and only a clip can be rated.
- **One command** (`sound.clip.rating`) and **one field** in the clip's
  payload. The MCP server now carries **246** commands, and the sound
  battery carries **37**.
- **The clip's kind says one thing more** — the rating, in the plan.

### What it does not settle

- **The rating is not shown.** A sound document is not a picture here,
  so a rating is a fact in the document and not a row of stars on a bar.
  The tree draws the stars.
- **The rating is not a shortcut.** The tree sets a rating with the
  keyboard — Cmd+Alt and a number — and here it is a command. A front
  end that wants the shortcut may call it; the command is the shortcut's
  only path, which is the point of the registry.
- **The group of a clip is still not here.** The tree's clip carries a
  **group** as well: a shared number that says which clips move
  together. It is the third mark, and the tree's own answer to it — the
  same number on each clip — is the answer this tree is most likely to
  change, because a group is an **edge** and not an id. That is the next
  slice.

## 199. The group of a clip

The tree's sound editor keeps a **group** on a clip: a shared number
that says which clips belong together. Two clips on two different tracks
can be in one group, and a group is not a folder — the clips stay where
they are. The v2 sound battery had no groups at all, and the last arc
named the group as the third mark and the answer this tree is most
likely to change.

### What was built

- The kind `sound.group` — a **group** of clips, which holds nothing
  and has no place: it is the name of a set.
- The edge `sound.grouped`, from a clip **to** its group, the way
  `core.on_layer` goes from a node to the layer it is on.
- The command `sound.clip.group` puts clips in a group **by name**: the
  group is made if it is not there, and the same name is the same group.
- The command `sound.clip.ungroup` takes clips out of their groups, and
  a group that is left with no clips in it is taken away.
- The command `sound.clip.group.read` reads the group of a clip — its
  name and its members — and the group of a clip and the members of a
  group are **derived** from the edges and said in the plan.
- A clip is in **one** group, and a group of **one** clip is refused.

### What the port found

- **The tree's group is a number, and a number is a handle nobody can
  say.** The tree's `clip.group` gives the named clips a fresh number,
  `clip.ungroup` takes it off them, and `clip.regroup` gives them the
  number that **most of them already had** — a person who groups three
  clips that were already together means to put them back together, and
  the tree has no other way of saying "the group they were in". It
  finds that group by **counting ids**: the group with the most members
  among the clips named wins. Here the group is a node, and a node has
  a name, so the same thing is said by saying the name — and the
  counting is gone.
- **The tree's group is not drawn and not named, and the one place it
  is seen is a file.** The tree's clip-group file writes the clips of a
  group out together into a `.scgrp` file, and the group is named by the
  file. A group in the tree is a number that only a writer ever sees.
  Here the group is a node in the document, so it has a name before
  there is any file to write.
- **The tree's group is a field on the clip, and here it is an edge.**
  The tree keeps the group number in the clip's own record, so putting
  a clip in a group is a rewrite of the clip, and the reader of a tree's
  file cannot tell a group from a field. Here the membership is an
  edge: the clip's own record is not touched, the group is a thing in
  the document, and the clip's group is derived from the edge it
  carries.
- **A group of clips is not the kit's group.** The kit's group is
  **structural**: a node with children, which the core walks into, and
  the studio's `node.group` puts the selected nodes *into* it. A sound
  clip that went into such a group would leave its playlist — and a
  playlist is where a clip's place in the timeline is kept — so the
  sound group holds **nothing**, its members point at it, and the clips
  stay on their own tracks. Two things with the same word and different
  meanings are two things, and the name says which is which.
- **The tree's group can have one clip in it and this tree's cannot be
  made that way.** The tree's `clip.group` refuses fewer than two
  clips, and so does this one; but the tree's group *ends up* with one
  clip in it when one of its clips is moved elsewhere, and the group
  stays. Here that is the same — a group with one clip in it is a group
  — and the group that is left with **none** is taken away, because a
  node that says nothing is litter.

### What it cost

- **Nine tests** in the sound battery (`tests/group.rs`): clips are put
  in a group, a group holds nothing and the clips stay, the name is the
  handle, a clip is in one group, a group of one clip is refused, a
  clip leaves its group, a clip in no group is left alone, the group is
  read, and only a clip can be grouped.
- **Three commands** (`sound.clip.group`, `sound.clip.ungroup` and
  `sound.clip.group.read`), **one kind** (`sound.group`) and **one edge
  kind** (`sound.grouped`). The MCP server now carries **249**
  commands, and the sound battery carries **40**.
- **The clip's kind says one thing more** — the group of the clip, in
  the plan — and the group says its members.

### What it does not settle

- **A group does not move its clips.** Nothing in the tree moves a
  group of clips together — the group is a marker and not a handle on
  the timeline — and nothing does here either. A command that moved
  every clip of a group at once would be a new thing, and the tree has
  no word for it.
- **The tree's clip-group file is not here.** The tree writes the clips
  of a group into a `.scgrp` file with their audio beside it; this tree
  writes sound (§ the render) and not groups.
- **The tree's "ungroup all" has no command of its own.** The tree can
  take every clip out of every group at once. Here the same thing is
  two steps — read the members of each group and ungroup them — because
  a group is named and its members are said in the plan, and a
  document-wide sweep is not a thing a command should do quietly.

## 200. The grid of the session

The tree's sound editor has a **grid** — a division of the bar — and a
command that puts a clip on it. The v2 sound battery had a tempo map
and a beat and no grid, so nothing here snapped a clip to anything,
and the sound battery's own notes said so.

### What was built

- The **grid**: a note value — a bar, a half, a quarter, an eighth, a
  sixteenth, a thirty-second, a sixty-fourth — and two flags,
  **dotted** and **triplet**.
- The grid is a setting of the **session**, with the sample rate and
  the timecode, so a file says what grid it was made on.
- The command `sound.grid.set` sets the grid and refuses a value that
  is not a note value, and the command `sound.grid.read` reads it: the
  step in ticks, the step in samples and the first lines.
- The command `sound.clip.quantize` puts a clip's **beat** on the
  nearest grid line and moves the clip so that its beat is there.
- The lines are counted **from the bar line**, so a bar of an odd
  number of beats does not make them drift.
- The **time model** grows the grid: `craftkit-sound-time` has a grid
  module with the note values, the step, the lines and the snap, and
  the tempo map has a public `bar_start_tick_at`.

### What the port found

- **The tree's grid is saved, in the wrong place.** The tree keeps the
  grid in the **edit state** — beside the tool, the edit mode and the
  selection — and the edit state is saved with the session, so the
  tree's file does say what grid the session was made on. It says it
  among the things the editor was doing, which is where a reader would
  not think to look. Here the grid is with the sample rate and the
  timecode: the file says what the grid is without saying what tool
  was selected when it was written.
- **The tree's grid is musical, and it is also four other things.** The
  tree's grid is a closed choice: a note value, a number of seconds, a
  number of frames or a number of samples. The last three are grids
  for a timeline that is not musical, and a sound document is a bar of
  beats — so here the grid is a division of the bar and nothing else.
  The tree's own default is musical too: a sixteenth.
- **The tree's grid lines restart at every bar, and it matters.** A bar
  of 3/4 has no even number of eighths in it, and lines counted from
  the start of the song would drift a quarter of a beat by the end of
  every odd bar. The tree counts them from the bar line, and so does
  this tree: it is the tree's rule and it is the right one.
- **The tree clamps a clip at zero and this tree refuses it.** The
  tree's quantise moves the clip so that its sync point is on the
  line, and the tree's move clamps a clip that would start before the
  beginning at zero — so the clip's sync point is *not* on the line
  after the move, and the command says nothing about it. Here the move
  that would put a clip before the beginning is **refused**: a clip
  whose beat is not on a line is not on the grid, and a clip that was
  quietly put somewhere else is a clip the caller did not ask for.
- **The tree's grid is a mode and a flag as well as a value.** The
  tree has an edit mode — shuffle, slip, spot, grid, relative grid —
  and a flag that says whether the edit cursor and the selections snap
  to the grid. Those are the behaviour of a **drag**, and v2 has no
  drag: the port is the part that is a fact about the document, which
  is the quantise command.

### What it cost

- **Ten tests** in the sound battery (`tests/grid.rs`): the grid is set
  and read, a grid that is not a note value is refused, a grid may be
  dotted or a triplet, the lines are counted from the bar line, a clip
  is put on the grid by its beat, a clip with no beat is put on the
  grid by its head, a clip on the grid does not change, a clip that
  would start before the beginning is refused, a locked clip is not put
  on the grid, and only a clip is put on the grid.
- **Three commands** (`sound.grid.set`, `sound.grid.read` and
  `sound.clip.quantize`) and **one setting** in the session's
  settings. The MCP server now carries **252** commands, and the sound
  battery carries **43**.
- **The time model grew** — a grid module in `craftkit-sound-time`,
  the note values, the step, the lines, the snap and the bar line — and
  the tempo map grew one public method, `bar_start_tick_at`.

### What it does not settle

- **The tree's absolute grids are not here.** A grid of seconds, of
  frames or of samples has no port: those are grids for a timeline
  that is not musical, and a sound document is a bar of beats.
- **The tree's grid drag modes are not here.** The tree's absolute and
  relative grid modes move a clip as it is dragged — the relative mode
  keeps the clip's offset from the line and moves it by whole steps.
  There is no drag in v2, and a drag is not a command: it is a
  movement with a button held down.
- **The grid is not shown.** A sound document is not a picture here, so
  the grid is a setting and a set of lines in the plan, and not a row
  of lines on a ruler.

## 201. The gain of a clip

The tree's sound editor has a gain on every clip, a command to set it,
a pair of commands to nudge it and a third to bypass it. The v2 sound
battery had a **gain field** in the clip's payload and nothing that set
it: every clip was played at unity, whatever the document said. The
field was there and the fact was unreachable.

### What was built

- The command `sound.clip.gain` sets the gain of clips, **absolutely**
  (`db`) or by a **change** (`delta_db`), and one of the two is
  needed.
- A gain outside the range — from minus one hundred and forty-four to
  thirty-six decibels — is **refused**, and the clips keep the gain
  they had. A change that would take a clip out of the range is
  refused as well.
- The command `sound.clip.gain.bypass` takes the gain out of the path
  and **keeps** it, and a bypass that is not named is toggled.
- The **render** and the **waveform** honour the bypass: a bypassed
  clip is as loud as it is.
- The clip's kind says the gain and the bypass in the plan.

### What the port found

- **The tree clamps a gain and this tree refuses it.** The tree's
  `clip.gain` takes any number and clamps it into the range: a gain of
  a hundred decibels becomes thirty-six, and a gain of minus a
  thousand becomes minus one hundred and forty-four. A gain asked for
  as a hundred decibels is not a gain, so here it is refused (§ the
  kit's own rule: a bound is not negotiable, and a refusal never
  truncates).
- **The tree's bypass is a reset and not a bypass.** The tree's
  `clip.gain_bypass` sets the clip's gain to zero and throws the old
  gain away — the clip plays as it is, and the gain that was there is
  gone for good. That is a **reset**, and the name says otherwise: a
  bypass is a thing that is taken out of the path, and the thing in the
  path is still there when the bypass is taken off. Here the gain is
  kept and a flag says that it is out of the path, so the clip plays
  as it is while the bypass is on and the gain comes back when it is
  taken off. The tree's version cannot do that: the gain it threw away
  is gone.
- **The tree's nudge is a change of a decibel.** The tree has two
  commands for it — nudge up and nudge down — and each is a change of
  one decibel. Here a change is a change: `delta_db` says how much,
  and the tree's two commands are a change of one and a change of
  minus one.
- **A field with no command is a fact nobody can change.** The gain
  was in the clip's payload from the first arc of the battery, and the
  renderer read it — and nothing could write it, so the field was
  always zero and the renderer was reading a thing that could not be
  anything else. A fact that no command can reach is not a fact in the
  document; it is a field.

### What it cost

- **Nine tests** in the sound battery (`tests/gain.rs`): the gain is
  set, the gain is changed by a delta, a gain outside the range is
  refused, a change outside the range is refused, a gain with nothing
  to say is refused, the gain is bypassed and kept, a bypass that is
  not named is toggled, the gain is heard, and only a clip has a gain.
- **Two commands** (`sound.clip.gain` and `sound.clip.gain.bypass`) and
  **one field** in the clip's payload. The MCP server now carries
  **254** commands, and the sound battery carries **45**.
- **Two places changed** — the render and the waveform — because a
  bypass that nothing asks is a flag and not a bypass.

### What it does not settle

- **The tree's bypass clears the curve and this one keeps it.** The
  tree's clip can carry a **curve** of gain over its length — and that
  curve is here, as a lane under the clip (`sound.lane.add` with the
  clip, not the track) — and the tree's bypass **clears** the curve as
  well as the level. Here the bypass takes the level out of the path
  and leaves the curve where it is, out of the path as well: a
  bypassed clip plays as it is, and taking the bypass off brings the
  curve back.
- **The tree's gain render is not here.** The tree can render a clip's
  gain to a file — the sound of the clip with the gain and nothing
  else — and this battery renders the whole document.

## 202. The conform of a clip

A recording is made at some tempo — the one the band was playing — and
a document may be at another. The tree's sound editor has a command
that **conforms** a clip to the document's tempo. The v2 sound battery
had a stretch and a tempo map and no way to say that a clip's material
belongs to a different tempo than the document's.

### What was built

- The command `sound.clip.conform` conforms a clip to the tempo of the
  document: the clip is played at the rate that makes the material
  fit, and the clip is as long as the material then lasts.
- The tempo of the **material** is a field of the clip, and it may be
  given: without it, the clip's own tempo is used, and the tempo of
  the document where the clip starts if the clip does not have one yet.
- The clip's **beat** follows the audio when the clip is conformed.
- A clip whose **content** is locked is refused, a clip with **no
  sound** has nothing to conform, and a tempo of the material outside
  the range is refused.

### What the port found

- **The tree keeps two facts about a clip outside the clip.** The
  tree's conform remembers the tempo of the material in the **edit
  state** — in a map of values, under a key made of the clip's id —
  and the length the clip had before it was ever conformed in another
  key beside it. So the two facts that say what a clip's material is
  are in a map that only the tree's editor knows about, and a reader
  of a tree's file cannot see them at all. Here the tempo of the
  material is a **field of the clip**, and it is in the plan.
- **The tree keeps the length it had before, and here it is not kept
  at all.** The tree remembers `len0` — the length of the clip before
  the first conform — so that a conform after a tempo change is the
  same conform whatever happened to the clip in between. Here that
  length is not kept: the material's length is the clip's length
  divided by its stretch, and it is worked out every time. The same
  answer, with one fact fewer to be wrong.
- **The tree's conform leaves the beat where it was, and the material
  moves away from it.** The tree's conform makes the clip twice as
  long and leaves the sync point at the sample it was at, so the
  material's downbeat is now twice as far from the beat that marks
  it. Here the beat moves with the audio, the way it moves when the
  clip is trimmed (§ the beat of a clip): the clip's timeline is what
  changed, and the beat is a place in it.
- **The tree skips what it cannot do, and here it is refused.** The
  tree's conform walks the clips and *skips* the ones that are locked
  or that have no audio, and it says how many it conformed and not
  which ones it did not. Here a clip that is locked, or that has no
  sound, is **refused by name** — a caller that asked for a clip to be
  conformed and got a list of the ones that were is a caller that has
  to work out what happened.
- **The tree clamps the ratio and this tree refuses it.** The tree
  clamps the conform's ratio into a window of 0.05 to 20, and the
  clip is conformed to the edge of the window. Here the ratio is a
  **stretch**, and a stretch outside the clip's own bounds is refused
  (§ the kit's own rule: a bound is not negotiable).

### What it cost

- **Eight tests** in the sound battery (`tests/conform.rs`): the clip
  is conformed, the material fits the tempo, the tempo of the material
  may be given, the beat follows the audio, a locked clip is not
  conformed, a clip with no sound is refused, a tempo outside the range
  is refused, and only a clip is conformed.
- **One command** (`sound.clip.conform`) and **one field** in the
  clip's payload. The MCP server now carries **255** commands, and the
  sound battery carries **46**.

### What it does not settle

- **The conform is not a time stretch.** The tree's elastic audio tracks
  stretch the clip and keep the pitch; here the conform is a
  **varispeed** — the material is played more slowly and the pitch
  falls with it, the way `sound.clip.stretch` does it. A pitch-
  preserving stretch is a different thing and it is not here.
- **The tree's pitch shift is not here.** The tree can remove the pitch
  shift of a clip and leave the length; here there is no pitch shift to
  remove, because there is no pitch-preserving stretch.

## 203. The name of a node

A node has a name. The kit's node had a name and **nothing could
change it**: `node.add` gives a node a name, and every other command in
the tree leaves the name alone. The tree this ports has a rename
command for **each** kind of thing — `clip.rename` and `track.rename`
— and this arc brings the one command that says it for every node.

### What was built

- The command `node.rename` renames **any** node — a shape, an
  appearance, a track, a clip, a group.
- The name is **trimmed**, and a name that is nothing once it is
  trimmed is **refused**: a node with no name cannot be found.
- Renaming a node to the name it already has changes nothing, and a
  command that changes nothing leaves no undo step behind.

### What the port found

- **The tree has a rename command for each kind and the kit had none
  at all.** The tree's sound editor renames a clip with `clip.rename`
  and a track with `track.rename`, and every other kind in every other
  tree has one too — the same command written again and again, once
  per kind. The kit has one command for every node, which is the same
  thing said once: the node is a node whatever it is, and the name is
  the node's.
- **The tree's rename is one for a selection and the kit's is one for
  a node.** The tree's `clip.rename` renames every clip in the
  selection, and the answer says how many clips there were. Here the
  command names the node it renames — and an agent that wants to
  rename five clips calls it five times and knows exactly which
  clips it renamed, which the tree's count does not tell it.
- **The tree truncates a long name and the kit has no limit.** The
  tree's `track.rename` keeps the first sixty-four characters of the
  name and says nothing about it, so a name that was asked for is not
  the name that is stored. The kit has no such limit — a node's name
  is a string and there is no bound on it — so a name here is the
  name that was asked for.
- **The tree refuses an empty name and so does this one.** The tree
  trims the name and refuses it when nothing is left, and a node with
  no name is a node nobody can find: the tree's own `node_by_name`
  would never find it.

### What it cost

- **Five tests** in the studio (`tests/rename.rs`): a node is renamed,
  the name is trimmed, a name that is nothing is refused, a node that
  is not there is refused, and renaming to the same name changes
  nothing.
- **One command** (`node.rename`). The MCP server now carries **256**
  commands.

### What it does not settle

- **There is still no command that renames a whole selection.** The
  tree's rename takes a selection, and here the command takes a node.
  A front end that renames a selection calls the command once for
  each node, and that is a decision for the front end.
- **A node's name is not its identity.** The id is the identity, and
  the name is what a person calls it by: two nodes may have the same
  name, and the kit does not pretend otherwise. The tree's rename has
  the same rule.

## 204. The copy of a node

The tree this ports has a command to duplicate each kind of thing — a
track is duplicated with its playlists and its clips, and everything
else is duplicated with whatever is under it. The kit had no command
to copy a node at all, and a document in which nothing can be copied
is a document in which every copy is made by hand.

### What was built

- The command `node.duplicate` copies **any** node and everything
  under it.
- The copy is a **new thing** with new ids: a track is copied with its
  playlists and its clips, and a group is copied with what is in it.
- The **edges** are copied, and an edge that joins two copied things
  joins the **copies**: the copy of a clip still plays the same source,
  and the copy of a track is still on the same layer.
- A copied child goes under the **copy** of its parent, and not under
  the thing it was copied from.
- The copy may be **named**, and without a name it is the node's own
  name with `.dup` on the end.

### What the port found

- **The tree's duplicate is one for a track, and the kit's is one for
  every node.** The tree's `track.duplicate` copies the track, its
  playlists and its clips, and gives the clips new ids while their
  contents stay. It also has flags — *without the playlists*, *without
  the automation*, *without the inserts*, *without the sends* — so a
  caller can ask for a copy with less in it. Here the copy is the whole
  node, and a caller who wants less copies less: the flags are a way
  of asking for a different thing, and the kit does not have a way of
  asking for a different thing here.
- **The tree's copy keeps the name of the thing it copied, with
  `.dup` on the end — and makes it unique.** The tree's
  `unique_track_name` adds a number when the name is taken, so the
  second copy of a track is called `drums.dup2` and the third
  `drums.dup3`. Here the name is the name that was asked for: the kit
  does not pretend that names are unique — a node's id is its
  identity and its name is what a person calls it by — so two copies
  may have the same name, and the tree's numbering was there to answer
  a question the kit does not ask.
- **The tree's copies are copies of the contents, and here the copy is
  a copy of the node.** The tree's duplicate clones the track's
  **struct**: the same fields, a new id, and the clips inside it with
  new ids. Everything a track knows about itself is in that struct,
  so the clone is complete. Here a node is a node in a document: it
  has a payload, a place, an appearance and **edges**, and the edges
  are the half of a node that is not in the node. So the copy copies
  the edges as well — and an edge that joins two copied things joins
  the copies, which is the part a struct clone cannot do: the tree's
  cloned clip points at the same source because the source was in the
  struct, and a copied group here points at the copied group.
- **The tree skips what it cannot copy and here it is refused.** The
  tree's duplicate walks the tracks and *skips* the ones that are not
  there, and it answers with the list of the ids it made. Here the
  node that is not there is **refused by name**, before anything is
  copied.

### What it cost

- **Six tests** in the studio (`tests/duplicate.rs`): a node is
  duplicated, what is under it is copied too, the edges are copied,
  the copy may be named, a node that is not there is refused, and — with
  the sound feature on — a track is duplicated with its clips and the
  copy's clip still plays the same source.
- **One command** (`node.duplicate`). The MCP server now carries
  **257** commands.

### What it does not settle

- **The tree's flags are not here.** The tree can copy a track without
  its playlists, without its automation, without its inserts and
  without its sends. Here the copy is the whole node.
- **A copy is not a reference.** The copy is a new thing: it has its
  own payload, and changing it changes nothing but itself. The tree's
  duplicate is the same, and the kit's **appearance** is the opposite
  — two things sharing one appearance is a reference, and a copy is
  not.

## 205. The group of tracks

The tree's sound editor has a second kind of group: a group of
**tracks**. The clip group of §199 is a set of things that belong
together; the track group is a **link** — the tracks in it are mixed
together, so a change to one track's level is a change to every track
in the group. The v2 sound battery had the clip group and no track
group, so nothing in it was ever linked to anything.

### What was built

- The group of tracks is the **same kind of node** as the group of
  clips, with two flags on it: **mix** says the group links its
  members' levels, and **active** says the group is on.
- The command `sound.track.group` puts tracks in a group **by name**,
  and the command `sound.track.group.set` sets the flags.
- The command `sound.track.ungroup` takes the group away and leaves
  the tracks where they were.
- The command `sound.track.fader` **asks the groups**: a change to the
  fader of a track in a group that mixes is a change to the fader of
  every member. (This arc, §207, is where the fader came: when this
  section was written the fader did not exist, and the group linked
  the **trim** — which is the tree's *input* gain, and the tree does
  not link it. The link moved when the thing it links arrived.)
- A group of **one** track is refused, the way a group of one clip is.

### What the port found

- **The tree has two group models and this tree has one.** The tree's
  clip group is a number in the clip's own record, and the tree's
  track group is an entry in a list of groups in the session — a name,
  a letter, the members, two flags, an active flag and a colour. Two
  models for one word, and neither of them is a thing in the document.
  Here a group is a **node**: the clips of §199 point at it and the
  tracks of this arc point at it, and the flags that only make sense
  for tracks are the group's own.
- **The tree's group has a letter because its group is a list entry.**
  The letter — a, b, c — is the tree's stable name for a group, and
  the tree's own test says it "never changes". Here the group is a
  node and its **id** is its stable identity; the letter was a
  substitute for an id, and the id was already there.
- **The tree's group is a list of members and here it is an edge.**
  The tree's group holds the ids of its member tracks, so a group is a
  second copy of the membership. Here the membership is an edge from
  each track to the group, so the group is derived from the edges —
  the same way the clip's group was, and for the same reason: a fact
  kept in two places is a fact that can be wrong in one of them.
- **The tree links the mix through a list of attribute names.** The
  tree's group carries a list — volume, mute, solo, rec — and a
  change to one of those attributes is a change to the whole group.
  Here the group says one thing: it **mixes**. The level is the thing
  that is linked — the **fader**, which is the tree's *volume* — and
  the phase, which is not a level, is not. (The trim is the tree's
  input gain, and the tree does not link it either; §207 says more.)
- **The tree skips what it cannot do and here it is refused.** The
  tree's `track.group` walks the tracks and skips the ones that are
  not there, and it answers with the id of the group it made. Here a
  track that is not a track is **refused by name**, and a group of one
  track is refused before anything is made.

### What it cost

- **Eight tests** in the sound battery (`tests/trackgroup.rs`): tracks
  are put in a group, a group of tracks mixes them, a group that is off
  links nothing, the mix can be taken off, a group is taken away and the
  tracks stay, a group of one track is refused, only a track is grouped,
  and the group is in the plan.
- **Three commands** (`sound.track.group`, `sound.track.group.set` and
  `sound.track.ungroup`) and **one command changed** — `sound.track.set`
  now asks the groups. The MCP server now carries **260** commands, and
  the sound battery carries **49**.

### What it does not settle

- **The tree's edit link is not here.** The tree's group has an
  *edit* flag: selecting one track of a group selects the whole group,
  so a move moves them together. That is a change to the **selection**,
  and the selection is the kit's here and the editor's there — so
  there is nowhere for the link to live yet.
- **The tree's mute and solo are not linked here.** The tree links
  them through its list of attributes; here the group links the level
  and says so, and the mute and the solo are the track's own.
- **The tree's group colour is not here.** A group in the tree has a
  colour of its own, and the kit's **appearance** is how a thing looks
  — so the group's colour is the appearance the group carries, and
  there is no command of the battery's own for it.

## 206. The elastic of a track

The tree's sound editor has an **elastic** mode: a track whose clips
are stretched and the pitch stays. It is the difference between a
record played at the wrong speed and a record played slowly, and the
second is the one a musician asks for. The tree keeps the difference
in **two** places — an algorithm on the track (`track.elastic`) and a
warp on the clip (`clip.elastic_properties`) — and the warp bakes the
material into a rendered file and remembers the original in the edit
state so that it can be put back. The v2 battery had a stretch, and
the stretch was always a varispeed: the pitch always fell.

### What was built

- The **algorithm** is on the track. The command
  `sound.track.elastic` sets it, with the names the tree uses —
  `polyphonic`, `rhythmic`, `monophonic`, `varispeed`, `x-form` — and
  the name `none` takes the elastic away. The algorithm is a **string**
  and not a variant, for the same reason the track's kind is: the
  kinds of a track are open here, and so are its algorithms.
- A clip on an elastic track is **stretched and the pitch stays**. The
  method is waveform-similarity overlap-add: the sound is cut into
  frames, each frame is written over the last, and the place each
  frame is read from is chosen so that it joins the frame before it as
  smoothly as it can. The frames are all of the same pitch, so the
  pitch is kept and the length is what changes.
- The **stretch is bounded** the way the tree's is: from a tenth to
  ten times. A stretch outside that is **refused** — by
  `sound.clip.stretch` and by `sound.clip.add` — and it is not pulled
  to the edge, because a caller that asked for a hundredfold stretch
  would be handed a tenthfold one and would not know.
- The algorithm `varispeed` says that the pitch **falls**, which is
  what an ordinary track does anyway.
- The **unwarp** is a stretch of one. There is no command that takes
  it off, because there is nothing to take off: the stretch is a fact
  and the material is derived from it.

### What the port found

- **The tree has two facts and this tree has one.** The tree's clip
  has a `stretch` — the varispeed — and the warp is a second thing
  that is rendered into a file and remembered in the edit state under
  `elastic.<id>.src`, `off`, `len` and `ratio`, so that
  `clip.remove_warp` can put the material back. Here a clip has **one**
  stretch, and the track says how it is played. The tree's own
  description of the difference says the same thing: "on Elastic Audio
  tracks (polyphonic/rhythmic/monophonic/x-form) the stretch is
  rendered pitch-preserving; elsewhere it is varispeed".
- **The tree's warp makes the clip longer and this one's does not.**
  The tree renders the material at `length × ratio`, replaces the
  clip's content with the rendered file and sets the clip's length to
  the result. Here the clip keeps its length on the timeline, and the
  source is read the way the stretch says: a clip stretched twice
  plays its source at half the speed, with the pitch kept. The length
  on the timeline is the clip's length in both, and what is derived
  from it is the sound.
- **The tree's algorithms are four pieces of work and here they are
  one.** `polyphonic`, `rhythmic`, `monophonic` and `x-form` are four
  ways of keeping a pitch, and each of them is a different piece of
  work. Here the name is kept — the name is the fact — and every one
  of them is rendered with the one method this has. The battery says
  so in its own header, because a name that is kept and a method that
  is not are different things and a reader should not have to guess
  which is which.
- **The tree clamps and this tree refuses.** The tree clamps the warp
  ratio to `0.05..20` and then refuses a computed ratio outside
  `0.1..10` when it renders. Here the bound is `0.1..10` on the
  stretch itself, and it is refused by the command that sets it.
- **The tree's remove-warp has no command here.** The tree needs one
  because its warp is a rendering: to undo it, the original source has
  to be put back from the edit state. Here there is no rendering and
  no edit state — a stretch of one is the undo, and it is the same
  command that set it.

### What it cost

- **Twelve tests** in the sound battery (`tests/elastic.rs`): the
  track carries its algorithm, `none` takes it away, an empty name is
  refused, a node that is not a track is refused, an elastic clip keeps
  its pitch, an ordinary clip drops it, `varispeed` drops it, an
  elastic stretch that is too large is refused, one that is too small
  is refused, an ordinary track may be stretched further, and a clip
  added to an elastic track is refused a stretch outside the bound and
  is given one inside it.
- **Seven tests** in the audio crate (`tests/stretch.rs`): the pitch
  stays, the loudness stays, a stretch of one is the sound, the
  channels stay apart, a stretch of half is shorter and the same
  pitch, a stretch out of range is clamped by the method, and an empty
  sound stays empty.
- **One command** (`sound.track.elastic`) and **two commands changed**
  — `sound.clip.stretch` and `sound.clip.add` now ask the track what
  it is. The audio crate gained `time_stretch`. The MCP server now
  carries **261** commands, and the sound battery carries **50**.

### What it does not settle

- **Four algorithms and one method.** The names are kept and every
  one of them plays the same way, because this tree has one
  pitch-preserving stretch and the tree has four. A third party may
  add a method, and the name is already there for it.
- **There is no warped material to look at.** The tree's warp leaves a
  rendered file in the session, and a clip can be pointed at it like
  any other source. Here the material is derived at render time from
  the source and the stretch, so there is nothing in the clip list to
  find.
- **The clip's own sound is not the tree's.** The tree's clip, once
  warped, is a clip of the rendered file; here it is a clip of the
  source with a stretch, and the sound of it is the source's, played
  as the stretch says.

## 207. The level of a track

The tree's mixer keeps **two** levels on a track and they are not the
same thing: the **trim** is the gain of the input, before the inserts,
and the **fader** is the level of the output, after them. This tree
had the trim and no fader, so a track's level could be automated and
never set — the only way to make a track quieter was to write a lane,
and a lane is a curve for something that moves.

### What was built

- The **fader** is a field of the track: `fader_db`, in decibels. The
  command `sound.track.fader` sets it, and a fader outside the tree's
  range — from minus one hundred and forty-four decibels to twelve —
  is **refused** and not pulled to the edge.
- The fader is the **default of the volume lane**, as it is in the
  tree: a lane with breakpoints overrides it, and a lane without one
  *is* the fader. So the sound of a track with a lane is the lane's,
  and the sound of a track without one is its fader's.
- The fader is the level of the track's **output**: it comes after the
  track's inserts, and a send may still take its copy from before it.
- The **group's link moves to the fader**: the tree's group links the
  **volume**, and the volume is the fader. The trim is the tree's
  input gain, and the tree does not link it — so `sound.track.set`,
  which sets the trim, no longer asks the groups, and
  `sound.track.fader` does.

### What the port found

- **The tree's trim and fader are two levels and this tree had one.**
  The tree's mixer has `trim_db` and `volume_db` side by side, and
  they are at different ends of the chain: the trim is before the
  inserts and the fader is after them. This tree had the trim and
  made the volume a lane, so the level of a track was a curve and
  never a number. The fader is that number.
- **The tree's fader is the lane's default.** The tree's lane is asked
  for its value with the fader as the default — `value_at(at, base)` —
  and the default is used when the lane has no breakpoints. So the
  fader is not a second level on top of the lane; it is what the lane
  falls back to. Here the fader is carried with the lanes, in the
  track's own automation, and it is used exactly when the lane has
  nothing to say.
- **The tree links the volume and this tree linked the trim.** The
  tree's group carries a list of the attributes it shares — the
  volume, the mute, the solo and the record — and the volume is the
  fader. The trim is not on that list, and the tree's `mix.trim` does
  not ask the group at all. This tree's group (§205) linked the
  **trim**, because the trim was the only level it had; the link
  moved to the fader in this arc, and §205's text was corrected in the
  same breath.
- **The tree clamps and this tree refuses.** The tree's fader is
  clamped into its range when a file is read and when it is set. Here
  a fader outside the range is refused, and the fader does not move.

### What it cost

- **Ten tests** in the sound battery (`tests/level.rs`): a track
  carries a fader, a fader is set, one that is too low is refused, one
  that is too high is refused, one that is not a number is refused, a
  node that is not a track is refused, the fader is heard, a volume
  lane overrides it, the trim is not linked by the group, and the
  fader is.
- **Four tests changed** in `tests/trackgroup.rs`: the group links the
  fader and not the trim, which is what the tree does.
- **One command** (`sound.track.fader`) and **one command changed** —
  `sound.track.set` no longer asks the groups. The MCP server now
  carries **262** commands, and the sound battery carries **51**.

### What it does not settle

- **The tree's mute, solo and record are not linked here.** The tree's
  group shares four attributes, and this tree shares one: the level.
  The mute and the solo are the track's own, and the record is the
  tree's arm button and has no place in a document.
- **The tree's fader is a nudge and a number.** The tree moves the
  fader in steps as well as setting it, and its nudge is a command of
  its own. Here the fader is a number and the number is set.

## 208. The pitch of a clip

The tree's sound editor can move a clip's **pitch**: the sound stays
as long as it was and it is another note. It is the other half of a
stretch — a stretch makes the sound longer and the pitch stays — and
the tree does it as one of its **audio-suite processes**: it renders
the material, replaces the clip's content with the rendered file, and
keeps the original in the edit state so that `clip.remove_pitch_shift`
can put it back. This tree had no pitch at all: a clip could be
stretched and could not be tuned.

### What was built

- The **pitch** is a number of **semitones** on the clip, and the
  command `sound.clip.pitch` sets it. Twelve semitones are an octave
  up and minus twelve an octave down, and zero is the pitch of the
  source.
- The shift runs from minus twenty-four semitones to twenty-four, and
  a shift outside that is **refused** and not pulled to the edge.
- The clip keeps its **length**: the sound is the same size and it is
  another note. The pitch and the stretch are two things, and a clip
  that has both is the stretch's note moved by the shift.
- The audio crate gains `pitch_shift` and `resample_ratio`: the sound
  is resampled as if it were played faster, which raises the pitch and
  makes it shorter, and it is then **stretched** back to the length it
  was — and a stretch keeps the pitch.

### What the port found

- **The tree's pitch shift is a rendering and this tree's is a fact.**
  The tree's `audiosuite.process` with `pitch_shift` renders the
  material into a new source, replaces the clip's content with it,
  sets the clip's stretch back to one and remembers the original
  source, the offset and the length in the edit state — so that
  `clip.remove_pitch_shift` can undo it. Here the pitch is a number on
  the clip and the sound is worked out when it is played: there is no
  rendered file, no edit state, and **nothing to remove** — a pitch of
  zero is the undo.
- **The tree's shift is two steps and this crate already had both.**
  The tree's `pitch_shift` stretches the sound by the factor — the
  pitch stays — and resamples it back to its length, which raises the
  pitch by the factor and puts the length back. The stretch was
  built in §206 and the resampler was built in §155, so the pitch
  shift is the two of them in a row.
- **The tree's shift is a clip and not a track.** The tree's
  `clip.remove_pitch_shift` undoes the shift of a clip, and the shift
  itself is a process over a clip's audio. Here it is a field of the
  clip for the same reason: the sound of one clip is that clip's.
- **The tree clamps and this tree refuses.** The tree's shift is
  clamped to twenty-four semitones when it is asked for and to
  forty-eight when it is done. Here the command refuses a shift
  outside twenty-four semitones, and the method's own clamp is the
  last line of defence and not the policy.

### What it cost

- **Ten tests** in the sound battery (`tests/pitch.rs`): a clip
  carries its pitch, a pitch is set on it, one that is out of range is
  refused, one that is not a number is refused, a node that is not a
  clip is refused, a locked clip is not shifted, the pitch is heard
  and the length is the same, a pitch down is heard, a pitch of zero
  is the sound of the source, and the pitch and the stretch are two
  things.
- **Three tests** in the audio crate (`tests/stretch.rs`): the shift
  raises the pitch and keeps the length, a shift down lowers it, and a
  shift of zero is the sound.
- **One command** (`sound.clip.pitch`). The MCP server now carries
  **263** commands, and the sound battery carries **52**.

### What it does not settle

- **A big shift loses the top of the sound.** The shift is made by
  resampling the sound as if it were played faster, and a resample
  takes away what was above the new half of the rate: an octave up is
  made from a sound whose top is gone. The tree's shift has the same
  loss for the same reason. A shift that keeps every harmonic is a
  different piece of work — a phase vocoder, or a granular one.
- **The tree's other processes are not here.** The audio suite also
  normalizes, reverses, inverts, removes the direct current and ramps
  the speed; each of them is a rendering in the tree and each of them
  would be a field here.

## 209. The processes on a clip

The tree's **audio suite** does four things to a clip's sound: it
reverses it, turns it upside down, brings it to a level, and takes the
direct current away. It does each of them by **rendering** the
material and replacing the clip's content with the rendered file — so
the tree's clip, after a process, is a clip of a new file, and the
sound it used to be is gone. This tree had no processes at all, and a
clip's sound could only be changed by stretching it, moving its pitch
or turning its level.

### What was built

- A **process** is a name and its parameters, and it is on the clip.
  The command `sound.clip.process` adds one and the command
  `sound.clip.process.remove` takes the last one of that name away.
- The processes are done in the order they were added, so a clip may
  have more than one and may have the same one twice — which is what
  the tree does when the same process is asked for twice.
- The four the tree has are here: **reverse**, **invert**,
  **normalize** (with a `target_db`) and **dc_offset**. The audio
  crate gains the four that do them.
- A process this battery does not know is **refused** — when it is
  asked for, and when the clip is played. A process that is quietly
  ignored is worse than no process at all: the document says the sound
  was changed and it was not.

### What the port found

- **The tree's processes are renderings and this tree's are facts.**
  The tree's `audiosuite.process` renders the material, adds it to the
  session as a new source, replaces the clip's content with it and
  sets the clip's stretch back to one — and the clip's gain, its
  fades and its place on the timeline stay where they were, because
  the tree's own test says the gain is kept. Here the process is on
  the clip and the sound is worked out when it is played: there is no
  new source, no rendered file, and a process that is taken off
  leaves the sound exactly as it was.
- **The tree's process list is closed and this tree's name is a
  string.** The tree's command lists the processes it knows and
  refuses the others. Here the name is a string, for the same reason
  the kinds of a sound document are strings here — a third party may
  name a process — but a name nothing knows is **refused** rather than
  ignored, so the openness is in the naming and not in the silence.
- **The tree's normalize takes a target and a flag.** The tree's
  normalizer may bring the sound to a level as a whole or channel by
  channel, and it clamps the target into its range. Here the level is
  the whole sound — the tree's own default — and a level outside the
  range is **refused**.
- **The tree's direct current is per channel and so is this one's.**
  A stereo sound may sit above zero in one channel and not in the
  other, and the mean of each is taken on its own.

### What it cost

- **Thirteen tests** in the sound battery (`tests/process.rs`): a
  process is on the clip, one that is not known is refused, one with
  no name is refused, a level outside its range is refused, a locked
  clip is not processed, the sound is reversed, the sound is upside
  down, the sound is at a level, the sound has no direct current, the
  processes are done in order, a process is taken off, one that is not
  there cannot be taken off, and the sound is the source again when
  the process is taken off.
- **Two commands** (`sound.clip.process` and
  `sound.clip.process.remove`) and **four functions** in the audio
  crate (`reverse`, `invert`, `normalize` and `remove_dc`). The MCP
  server now carries **265** commands, and the sound battery carries
  **54**.

### What it does not settle

- **The tree's other processes are not here.** The audio suite also
  ramps the speed over time and generates a signal; a ramp is a
  curve over the clip and a generator is a new source, and both are
  more than a name and its parameters.
- **The tree's plugins are not here either.** The audio suite runs the
  tree's own plugins over a clip — an equaliser, a compressor, a
  reverb — and those are the same inserts that the kit already has on
  a clip (§166).

## 210. The pan of a track

The tree's mixer keeps a **pan** on every track: the balance of the
track's sound in the mix. This tree had a pan **lane** and no pan of
its own, and the lane's law was the tree's constant-power one — which
is a law for placing a mono sound in a stereo field, and takes three
decibels off a stereo sound in the middle.

### What was built

- The **pan** is a number on the track, from minus one to one, and the
  command `sound.track.pan` sets it. A pan outside that range is
  **refused** and not pulled to the edge.
- The pan is the **default of the pan lane**, as the fader is the
  default of the volume lane: a lane with breakpoints overrides it,
  and a lane without one is the pan.
- The law is the tree's **balance** law: the middle is the middle, and
  the sound is not quieter there than it is at the side. The send's
  pan uses the same law, so a copy and the track it came from are
  balanced the same way.
- The pan is applied when the track is **routed**: a track that feeds a
  bus is balanced before it reaches the bus, and a copy that a send
  takes is balanced by the **send's** own pan and not by the track's.

### What the port found

- **The tree's pan is a vector and this tree's is a number.** The
  tree's mixer keeps a pan per channel — a mono track has one panner
  and a stereo track has two, hard left and hard right by default —
  and the tree's command sets one of them at a time, with an `index`.
  Here a track has one pan and it balances the sound. The tree's
  default and this one's are the same thing: hard left and hard right
  is no pan at all.
- **The tree has four laws and this tree has one.** The tree's pan
  law is a setting of the session: constant power (three decibels off
  in the middle), a compromise (four and a half), constant voltage
  (six), or balance (none). The first three are laws for *placing* a
  mono sound in a stereo field. A stereo sound that is already in two
  channels is not placed by them — it is balanced, and the balance
  law is the one that does that without taking the level off. The tree
  has that law too; it is the one the tree calls `Zero`.
- **The tree's constant-power law was in this tree's pan lane.** The
  lane took three decibels off a stereo track in the middle, which is
  the placement law applied to a sound that is not being placed. The
  lane is now the balance law, and the middle is the middle.
- **The tree's pan is applied when the track is routed, and here it
  was not.** A track that feeds a bus is heard on that bus, and this
  tree balanced it only when it reached the main mix — so a track on a
  bus was never balanced at all. It is balanced before it reaches the
  bus now.
- **The tree's group does not share the pan.** The tree's group shares
  the volume, the mute, the solo and the record, and the pan is not
  among them; so the pan is the track's own here too.

### What it cost

- **Ten tests** in the sound battery (`tests/pan.rs`): a track carries
  its pan, a pan is set on it, one that is out of range is refused,
  one that is not a number is refused, a node that is not a track is
  refused, the pan is heard, a pan in the middle is the middle, a pan
  lane overrides the pan of the track, a routed track is balanced
  before it reaches the bus, and the pan is not linked by the group.
- **One command** (`sound.track.pan`) and **one law changed**: the
  pan of a lane and the pan of a send are the balance and not the
  constant power. The MCP server now carries **266** commands, and the
  sound battery carries **55**.

### What it does not settle

- **A mono sound is not placed.** This tree's mix has the shape of the
  media — a document of mono sources is a mono mix — so there is no
  stereo field for a mono sound to be placed in, and a pan of a mono
  track does nothing. The tree's session has a channel format of its
  own, and that is where the field would come from.
- **The tree's surround panner is not here.** The tree's mixer has a
  pucker for a surround field, with a divergence and a share for the
  centre speaker, and an Ambisonics encoder under it. That is a
  loudspeaker layout and not a balance.

## 211. The stems of a session

The tree's **bounce** writes one file for each track: the mix with
one track in it. It is how a mix is handed to somebody else — each
part on its own, so that it can be worked on again. This tree could
render the document into one file and had no way to render it into
several.

### What was built

- The command `sound.render.stems` writes **one file for each track**
  into a directory, and the directory is made if it is not there.
- A stem is the mix with **one track in it**: the track is soloed and
  the mix is worked out again, so a stem carries the master faders and
  the metronome with it, and a stem of a track that feeds a bus carries
  the bus.
- Each file is named after its track, and a name that is taken gets a
  number after it: two tracks with the same name are two files and not
  one file written twice.
- A **muted** track is not bounced: a stem of silence is a file nobody
  asked for. The mute is a **lane** here and not a field of the track,
  so a muted track is one whose mute lane says so at the start.
- The renderer was **split**: the mix is now worked out by a function
  of the document alone, and the render and the stems are two ways of
  writing that one mix.

### What the port found

- **The tree bounces the mix again for every track and this tree
  bounces the same mix.** The tree's `bounce_stems_with` clones the
  session, solos the track and renders the whole mix — a render per
  track, and the mixer run from the beginning each time. Here the mix
  is a **function of the document**: the render writes it and the
  stems write it with one track in it, and the two are the same sound
  because they come from the same mixer.
- **The tree's stems are tracks and this tree's are tracks too.** The
  tree bounces the audio, instrument and MIDI tracks that are neither
  inactive nor muted; a bus is not a stem in the tree and it is not one
  here. What a stem *carries* is a different thing: the tree's stem of
  a track that feeds a bus carries the bus, because the bus is in the
  mix, and so does this one.
- **The tree names the files after the session and the track.** The
  tree's file is `<session> - <track>.wav` and the name is made safe
  for a file system: the letters and the numbers and a few marks are
  kept and the rest becomes an underscore. Here the name is the
  track's alone, and the same rule makes it safe.
- **The tree's mute is a field and this tree's is a lane.** The tree
  skips a track whose mixer says it is muted; here the mute is a lane
  under the track, so a muted track is one whose mute lane says so at
  the start.

### What it cost

- **Eight tests** in the sound battery (`tests/stems.rs`): each track
  is bounced into a file of its own, a stem is the mix with one track
  in it, the mix is both of them, the directory is made if it is not
  there, two tracks with the same name are two files, a muted track is
  not bounced, a directory that is not a directory is refused, and the
  stems are the render of the document.
- **One command** (`sound.render.stems`) and **one refactor**: the mix
  is now worked out by `mix_of`, a function of the document alone, and
  the renderer writes it. The MCP server now carries **267** commands,
  and the sound battery carries **56**.

### What it does not settle

- **The tree's stems carry a format and a depth.** The tree's bounce
  writes WAV, AIFF or FLAC, at sixteen, twenty-four or thirty-two bits
  of float, with dither and a fold-down to stereo. Here a stem is a
  WAV at the session's rate, and the file the kit writes is the file
  it has.
- **The tree bounces a range.** The tree's bounce takes the start and
  the end of what is bounced; here the whole document is bounced, as
  the render does.

## 212. The separation of a clip

The tree's editor can **separate** a take: every clip that crosses a
place on the track becomes two clips, and the two are the same sound
with a line between them. It is how a take is cut into parts — a
drum loop into its hits, a phrase into its notes. This tree had no way
to cut a clip in two: a clip was one thing from its start to its end.

### What was built

- The command `sound.clip.separate` cuts a clip at a place **inside**
  it, and the clip becomes two clips under the same take.
- The two pieces are the **same sound**: the first keeps the head of
  the clip and the second keeps the tail, and the second starts later
  in the source by the distance the cut made — **divided by the
  stretch**, because a stretched clip's source is read more slowly.
  (A **MIDI** clip's notes are divided at the cut too, and this was
  not true when this section was written: §232 says what was wrong
  and what fixed it.)
- The **beat** moves with the audio it is in: a beat that the cut left
  in the other piece is not a beat of this one.
- The **fade** at the cut is dropped: the tree drops it too, because a
  fade that ended at the cut would be a fade of nothing.
- A place that is **not inside** the clip is refused — at its edge and
  outside it alike — because there is nothing to separate.

### What the port found

- **The tree separates a place on a track and this tree separates a
  clip.** The tree's separate takes the track and the place and splits
  every clip that crosses it, so one command can cut a whole take.
  Here one clip is separated at one place. The tree's own editor does
  the same thing one clip at a time, and the place is inside the clip
  there too: the tree's `split_clip` refuses a place at the clip's
  edge or outside it.
- **The tree's cut is two trims.** The tree's `split_clip` trims the
  clip to end at the place and trims a copy to start there — and the
  trims move the content and the beat, because that is what a trim
  does. Here the two pieces are made directly, and the content moves
  by the delta **divided by the stretch**, the same fact the trim
  command uses (§185).
- **The tree drops the fade at the cut and so does this.** The tree's
  split sets the left piece's fade-out and the right piece's fade-in
  to nothing, because a fade that ended at the cut would be a fade of
  nothing.
- **The tree's pieces are named by an auto-rename scheme and this
  tree's are not.** The tree gives the new piece the next name of the
  scheme the clip belongs to, and the piece keeps the clip's name when
  there is no scheme. Here the piece keeps the name, and the tree's
  **auto-rename** — a set of names that count up as a take is cut — is
  not built.
- **The tree separates on transients and on the grid as well.** The
  tree can find the places itself: a sound's transients, or the lines
  of the grid. Here the place is given, and the finding of the places
  is a piece of analysis this tree does not have.

### What it cost

- **Nine tests** in the sound battery (`tests/separate.rs`): a clip is
  separated into two, the second piece starts later in the source, a
  stretched clip is separated by the stretch, the two pieces are heard
  as one, a place that is not inside the clip is refused, a place at
  the edge is refused, a locked clip is not separated, a node that is
  not a clip is refused, and the beat moves with the audio.
- **One command** (`sound.clip.separate`). The MCP server now carries
  **268** commands, and the sound battery carries **57**.

### What it does not settle

- **The tree finds the places itself.** Separating on transients needs
  the sound to be looked at, and separating on the grid needs the grid
  to be asked; here the place is given.
- **The tree's auto-rename is not here.** A set of names that count up
  as a take is cut is a fact about the take, and this tree has one
  name for a node and a command that changes it.

## 213. The mute of a track

The tree's mixer keeps a **mute** of its own beside the mute lane: a
mute is a switch on the track, and the lane is a curve over it. This
tree had the lane and no switch — a track could be silenced only by a
lane under it, and a lane is a curve for something that moves.

### What was built

- The command `sound.track.mute` mutes a track and lets it be heard
  again. The mute is a field of the track.
- The **mute lane** is the mute's default, as the volume lane is the
  fader's default and the pan lane is the pan's: a lane with
  breakpoints overrides it, and a lane without one is the mute.
- The mute is one of the things a **group** shares: the tree's group
  shares the volume, the mute, the solo and the record, and the mute
  is one of them — so a change to the mute of a track in a group that
  mixes is a change to the mute of every member.

### What the port found

- **The tree's mixer keeps a mute and a lane, and here the lane was
  the mute.** The tree's `Mixer` has a `mute` of its own and an
  automation lane over the mute parameter, and the tree asks the lane
  for its value with the mute as the default: a lane with breakpoints
  overrides it, and a lane without one is the mute. Here the mute was
  the lane alone, so the only way to silence a track was to write a
  curve of ones.
- **The tree's group shares the mute and this tree's group did not.**
  The tree's group carries a list of the attributes it shares — the
  volume, the mute, the solo and the record — and the fader of §207 is
  the volume and the mute of this arc is the mute. The solo and the
  record are still not shared: the solo is a rule about the whole
  document and the record is a button of the tree's arm.
- **The tree's mute is a switch and a lane, and here the switch is
  the default.** The two are not the same thing: a switch is a
  decision and a lane is a movement, and the tree's own mixer asks
  the lane first and the switch second. Here the switch is the lane's
  default, and the same shape as the fader and the pan.

### What it cost

- **Six tests** in the sound battery (`tests/mute.rs`): a track
  carries its mute, a mute is set on it and taken off, a muted track
  is not heard, a mute lane overrides the mute of the track, the mute
  is linked by the group, and a node that is not a track is refused.
- **One command** (`sound.track.mute`). The MCP server now carries
  **269** commands, and the sound battery carries **58**.

### What it does not settle

- **The solo and the record are not shared by the group.** The tree
  shares four attributes and this tree shares two: the level and the
  mute. The solo is a rule about the whole document — one soloed track
  silences the rest — and sharing it would be a rule about the group
  as well, which is a different thing.
- **The mix has the shape of the media.** The tree's session has a
  channel format of its own and its mixer builds a matrix from the
  source's channels to the destination's; here the mix has the shape
  of the media, so a mono document is a mono mix and the pan of a mono
  track does nothing. The matrix is a redesign of the mixer and not an
  addition to it, and it is the next thing the tree has that this
  tree cannot say.

## 214. The solo-safe track

The tree's mixer keeps a **solo-safe** flag beside the solo: a solo
silences the other tracks, and a solo-safe track is not silenced by
it. It is the flag that says *this one stays* — a reference, a click, a
talk track — when somebody listens to one thing on purpose. This tree
had the solo and no way to exempt a track from it.

### What was built

- The command `sound.track.solo.safe` makes a track solo safe and
  takes it back.
- A solo silences the other tracks as it did before, and a solo-safe
  track is **not** silenced by it — neither its own sound nor the copy
  that a send takes of it.

### What the port found

- **The tree's flag is beside the solo and this tree's is a command of
  its own.** The tree's `mix.solo_safe` is a switch on the track, like
  the mute of §213, and the mixer asks it when it silences the others:
  a track is silenced by a solo unless it is soloed itself or it is
  solo safe. Here the flag was missing and the mixer silenced every
  track that was not soloed.
- **The tree has a third case and this tree has two.** The tree's
  mixer also keeps a track that **receives a solo** — a track fed by a
  soloed track through a bus — and that one is not silenced either.
  Here a bus is a bus and the question of what it receives is the
  routing's, so the flag is the only way to stay.

### What it cost

- **Six tests** in the sound battery (`tests/solosafe.rs`): a track
  carries its solo safe, a solo silences the other tracks, a solo-safe
  track stays, a track that is not solo safe is silenced, the solo
  safe can be taken off, and a node that is not a track is refused.
- **One command** (`sound.track.solo.safe`). The MCP server now
  carries **270** commands, and the sound battery carries **59**.

### What it does not settle

- **The tree's "receives a solo" is not here.** A track fed by a
  soloed track through a bus is not silenced in the tree, and here it
  is: the flag is the only way to stay.

## 215. The separation of a clip on the grid

The tree's editor separates a take **on the grid**: the clip is cut
at every line of the grid it crosses, and the pieces are the bars or
the beats the grid says they are. The separation of §212 takes a
place; this one takes the **lines**.

### What was built

- The command `sound.clip.separate.grid` cuts a clip at every **line
  of the session's grid** that it crosses. The grid is the session's
  (§200) and the lines are musical, so they move with the tempo and
  follow the meter.
- The number of lines is **bounded**: a clip that crosses more lines
  than the bound is **refused**, and not cut at the first thousand of
  them. A clip cut at a thousand lines is a clip nobody meant to make.
- The cut itself is §212's cut: the pieces are the same sound, the
  second starts later in the source by the distance the cut made, and
  the fade at the cut is dropped.

### What the port found

- **The tree separates a range on a track and this tree separates a
  clip.** The tree's `separate_on_grid` takes the tracks and the
  range, reads the grid lines of the range — bounded at ten thousand
  — and separates every clip that crosses each of them. Here one clip
  is separated at the lines it crosses, and the lines are the clip's
  own span.
- **The tree truncates the lines and this tree refuses.** The tree
  reads at most ten thousand lines and separates at the ones it got;
  a clip that crosses more lines than that is cut at the first ten
  thousand and the rest of it is not cut at all. Here the bound is a
  thousand lines and a clip that crosses more is **refused** — a
  refusal never truncates (§ the kit's own rule), and a clip that was
  cut at some of its lines and not at the others is a clip that says
  something false about its own length.
- **The tree's lines are read from its edit state and this tree's
  from the session.** The tree's grid lives beside the tool and the
  edit mode (§200), and here it lives with the sample rate and the
  timecode — so the lines a clip was cut on are in the document and
  not in the editor's state.
- **The tree's separation is a track's and this tree's is a clip's.**
  The tree's cuts every clip that crosses the line, so one command can
  cut a whole take; here the clip is named, because a clip is the
  thing the command is about.

### What it cost

- **Five tests** in the sound battery (`tests/separategrid.rs`): a
  clip is cut at every line it crosses, the pieces are the same sound,
  a clip that crosses no line is not cut, a locked clip is not cut,
  and a node that is not a clip is refused.
- **One command** (`sound.clip.separate.grid`) and **one cut** shared
  with §212 — the cut is now a function of its own, and the two
  commands are two ways of saying where the cuts are. The MCP server
  now carries **271** commands, and the sound battery carries **60**.

### What it does not settle

- **The tree finds the places itself.** Separating at transients needs
  the sound to be looked at, and the tree's sensitivity is a number
  the caller gives; here the places are the grid's, which are known
  without looking at anything.
- **The tree separates at a range and not at a clip.** A take of
  twenty clips is cut in one command in the tree and in twenty here.

## 216. The auto rename of a clip

The tree's sound editor has an **Auto Rename** scheme: a name, a
number and a suffix, and the clips that editing separates from a clip
with a scheme are named from it. "Kick" cut in four becomes "Kick",
"Kick1", "Kick2" and "Kick3" — and the numbers keep counting, so the
next clip separated from any of them is "Kick4".

### What was built

- The command `sound.clip.autoname` sets the scheme of clips, and
  `sound.clip.autoname.read` reads it back with the **next** name it
  would give.
- A **scheme is a node** (`sound.autoname`) and the clips in it point
  at it with an edge of kind `sound.autonamed` — the same shape as a
  group (§199).
- The **piece after a cut** takes the scheme's next name and the scheme
  counts up; the piece **before** the cut keeps the name it had. The
  cut is §212's cut, so a clip separated on the grid (§215) is named
  as well.
- The name is made of the **name**, the **zeros**, the **number** and
  the **suffix**, in that order: "SFX", one zero, twenty-four and
  ".Reel1" is "SFX024.Reel1".
- A scheme that **no clip is in** is removed, and a scheme is taken
  away by the same command without a name.

### What the port found

- **The tree's scheme is a number in a map and this tree's is a
  node.** The tree keeps the schemes in the session, keyed by a number,
  and each clip holds the key of the scheme it is in. Here the scheme
  is a node with a payload, and the clips **point at it** — so the
  membership is an edge, a clip's record is not rewritten to change
  its scheme, and a scheme that no clip uses is **pruned** instead of
  being a key in a map that says nothing.
- **The tree's key makes the scheme shared and so does the edge.** The
  clips separated from one clip share the scheme and count up one
  number sequence. That is the reason the tree has a key at all, and
  the reason the scheme is a thing here: the pieces of a clip are in
  the scheme too, so a cut of a piece counts on in the same sequence.
- **The tree's scheme has an id and this tree's has none.** The tree's
  id is "to tell this scheme from another one under the same key after
  an undo or in another session, for pastes from the clipboard". The
  clipboard is here now (§247), and the schemes it holds are told
  apart by their **nodes**: a node is the thing, and the tree's id is a
  key under a key.
- **The tree's zeros are zeros and not padding.** The tree writes the
  name, then that many zero characters, then the number, then the
  suffix — "SFX", one zero, twenty-four and ".Reel1" is "SFX024.Reel1",
  and not "SFX24.Reel1" with the number padded to some width. The
  same rule here, because a reader of a name should not have to know
  how wide the number is meant to be.

### What it cost

- **Nine tests** in the sound battery (`tests/autoname.rs`): the piece
  after a cut takes the next name, the name is made of its four parts,
  the scheme counts up, two clips share one sequence, the scheme is
  taken away without a name, a scheme no clip is in is removed, the
  number and the zeros and the suffix need a name, a number outside
  the bound is refused, and a node that is not a clip is refused.
- **Two commands** (`sound.clip.autoname` and `sound.clip.autoname.read`)
  and **one kind** (`sound.autoname`). The MCP server now carries
  **273** commands, and the sound battery carries **62**.

### What it does not settle

- **The tree names the clips it strips as well.** The tree's strip
  silence names the pieces it keeps and leaves the ones it removes
  alone, in time order; here only a cut names a piece, because a cut
  is the only thing that makes a piece.
- **The tree's scheme survives a paste.** A copy carries its scheme
  with it, and the copy pasted into another session brings the scheme
  along and counts on from it. The clipboard is here now (§247), but it
  carries the **clips** and not the schemes, so a pasted clip is not in
  a scheme and its name is the name of the clip it was made from.

## 217. The MIDI library

MIDI is a **sequence of events on a timeline**: a note, or a change of
something that is not a pitch. The tree's sound editor has all of it —
a MIDI clip holds a sequence of notes and controllers, and the tree
reads and writes Standard MIDI Files — and v2 had none of it: there
was no note in the whole of v2, and no MIDI file.

### What was built

- The crate `craftkit-midi`: the **model** — a `Note`, a `CtrlEvent`
  and a `Sequence` of them — and the **file**: `read_smf` and
  `write_smf`, the Standard MIDI File, in the public SMF 1.0
  specification.
- The ticks are counted at **960 a quarter note**, and every file is
  converted to them when it is read, so a tick is a tick everywhere.
- The reader is **patient** with what the format allows: a running
  status, a note-on of velocity zero as a note-off, a note-off with no
  note-on (ignored), two notes of the same pitch on top of each other
  (matched first in, first out) and a note that is never let go
  (closed at the end of the track). It is not patient with what the
  format does not allow: a header that is not one, a chunk longer than
  the file, a number longer than four bytes, a data byte with no
  status to run on. Each of those is a **refusal with a name**.
- The file is **hostile**: the number of events and the number of
  tracks are capped, and the writer **refuses** a fact it cannot
  write — a tempo of zero, a meter whose denominator is not a power of
  two.

### What the port found

- **The tree's MIDI is a crate and this tree's is a crate too.** The
  tree has `soundcraft-midi`: the model, the file, the **editing
  operations** and the display names. Here the model and the file are
  built, and the operations and the names are not — a note may be read
  and written, and moving one is the tree's operation and not yet
  here.
- **The tree's ticks are the model's ticks and so are these.** The
  tree counts at 960 a quarter note and converts every file to it when
  it is read, so that a file's own division does not change what a
  tick means. Here the same: the division in the file is read and
  converted, and a SMPTE division — frames a second — is approximated
  at a hundred and twenty beats a minute, because a file does not say
  how fast it is played.
- **The tree's errors are the errors of a `thiserror` enum and these
  are the kit's.** The tree's `MidiError` is a list of named
  refusals; here it is the same list, written the way the kit writes
  its own — a `Display` and a `std::error::Error`, and no `thiserror`,
  because the kit's crates carry no dependencies.
- **The tree's file is not an `Eq` and this one is not either.** A
  tempo is a number of beats a minute, and a number of beats a minute
  is a float; a file that holds one cannot be compared with `==`. The
  tree's `Smf` says the same, and here the reason is written down
  where the type is.

### What it cost

- **Fifteen tests** in the new crate (`tests/midi.rs`): a file that is
  written is a file that is read back, an empty file, the ticks, the
  controllers, the tempos and the meters, a file that is not a MIDI
  file, a truncated file, a number that is too long, a chunk longer
  than the file, notes that overlap, a note that is never let go, a
  tempo that is not a tempo, a meter that is not a meter, the
  duplicates and the end of a sequence.
- **One crate** (`craftkit-midi`), at **L0** — it depends on nothing,
  and the layer table says so.

### What it does not settle

- **The notes are not in a document yet.** A MIDI file may be read and
  written, but a clip does not hold notes: the sound battery has no
  MIDI clip, and the library is not yet under anything.
- **The editing operations are not here.** Quantising a note, moving
  it, changing its velocity and splitting it are the tree's
  operations, and they are arithmetic over the notes — but they are
  the tree's, and they are not in this crate yet.
- **The names are not here.** The tree names a note ("C3"), a drum and
  a program; a name is for a person to read, and nothing here is read
  by a person yet. (There is now: §237 built the names, and this
  sentence was written before it did.)

## 218. The MIDI of a clip

The tree's sound editor has a **MIDI clip**: a clip whose content is a
sequence of notes and controllers, read from and written to a Standard
MIDI File. The library was built in §217, and the clip that holds the
notes was not: a clip in v2 was a clip of sound and nothing else, and
the notes had nowhere to live.

### What was built

- The clip's **content** is a field in the clip's payload: `midi` for a
  clip of notes, and nothing for a clip of sound. That is the whole of
  the tree's closed enum of three kinds of clip — audio, MIDI and
  video — and it is a **string** here, so a third party may add a kind
  of content without touching the core.
- The **notes** and the **controllers** are in the clip's payload, and
  the commands are `sound.clip.midi.import`, `sound.clip.midi.export`
  and `sound.clip.midi.read`.
- The **length** of the clip is **derived**: it is as long as the notes
  need, at the tempo of the document where the clip starts.
- The file is **hostile**: a file that is not a MIDI file is refused,
  a file with no notes is refused, and the number of notes a clip may
  hold is bounded.

### What the port found

- **The tree's content is a variant and this tree's is a string.** The
  tree matches on the three kinds of a clip in the mixer. Here the
  clip says what its content is, and a kind this battery does not know
  is not a kind it can read: the clip's payload is **refused** rather
  than believed.
- **The tree's note is a value in a sequence and so is this tree's.**
  The tree's note has no identity beyond its pitch, its place and its
  length, and the sequence is a value in the clip's record. Here the
  notes are in the clip's payload as well — a note is not a node,
  because a node is a thing with a life of its own and a note has
  none.
- **The tick of a note is the kit's tick.** The MIDI library counts in
  nine hundred and sixty ticks a quarter note, and so does the sound
  battery's tempo map (§19). So a note's place in a clip and a clip's
  place on the timeline are two readings of the same scale, and the
  length of a MIDI clip is worked out from its notes at the tempo of
  the document — and not at a tempo of the file's own.
- **The tree's file keeps its own tempo and this tree's does not.** A
  MIDI file may carry a tempo, a meter and a key signature. The
  library reads them (§217), and the import reads the **notes** at the
  tempo the **document** says: a second tempo map would be a second
  truth about the same timeline, and the document already has one.
- **A MIDI clip is not sound, and it says so.** The tree plays a MIDI
  clip through a synthesiser; there is no synthesiser here, so a MIDI
  clip has no source and the render leaves it alone. It is silent, and
  it is silent **honestly**: what the clip holds is notes, and the
  notes are not a sound until something makes them one.

### What it cost

- **Eight tests** in the sound battery (`tests/midi.rs`): a file is
  imported as a clip that holds notes, the length of the clip is
  derived from the notes, the notes are read, the MIDI is exported and the file holds
  the same notes, a file that is not a MIDI file is refused, a file
  with no notes is refused, a clip that is not a MIDI clip is refused,
  and a MIDI clip is not heard.
- **Three commands** (`sound.clip.midi.import`, `sound.clip.midi.export`
  and `sound.clip.midi.read`). The MCP server now carries **276**
  commands, and the sound battery carries **65**.

### What it does not settle

- **The notes are not edited.** Quantising a note and moving it are the
  tree's operations, and they are arithmetic over the notes — but they
  are the tree's, and a clip's notes may be read and written here and
  not yet moved.
- **The tracks of a file are merged.** A clip holds one sequence, and a
  file may hold many: the import puts the notes of every track into the
  one clip. A file of three parts is a clip of one sequence, and the
  parts are not kept apart.
- **There is no synthesiser.** A MIDI clip is silent, and the tree's
  is not — the tree plays it. Making a sound out of notes is a bigger
  piece of work than reading them.

## 219. The editing of the notes of a MIDI clip

The tree's sound editor moves notes: it quantises them onto a grid,
transposes them, changes how hard they are struck and how long they
last. The clip that holds the notes was built in §218, and the notes
could not be touched.

### What was built

- The **library** grows an **operations** module: the quantise, the
  transpose, the velocity and the length, and the split by pitch. An
  operation works in place on a list of notes and never changes how
  many notes there are.
- The **battery** grows three commands: `sound.clip.midi.quantize`,
  `sound.clip.midi.transpose` and `sound.clip.midi.velocity`.
- The quantise puts the notes on the **session's grid** (§200) — the
  lines are musical, so they move with the tempo — and the clip keeps
  its length: the room the notes had is the room they have. (This was
  written the other way round at first — the length was worked out
  again from the notes — and §223 says why it changed.)
- A pitch or a velocity outside the MIDI range is **refused**, and the
  notes are not changed at all.

### What the port found

- **The tree clamps and this tree refuses.** The tree's transpose
  pushes a note that would leave the range onto the end of it, and its
  velocity does the same. Here both are refused and no note is moved:
  a note that was asked for and is not there is a note the caller does
  not know about — the same rule the gain follows (§201).
- **The tree's quantise is random and this one is not.** The tree's
  quantise has a random component, so that the same quantise twice
  does not give the same notes. Here it does: a note's place in a
  document is a fact, and a fact that changes each time it is asked
  for is not a fact.
- **The tree's quantise takes a grid of its own and this one takes
  the session's.** The tree's grid is a number of ticks; the kit's
  grid is a division of the bar, and the lines are counted from the
  bar line and move with the tempo. One grid in the kit, and the notes
  are put on it.
- **The tree's note names are not here.** The tree names a note
  ("C3"), a drum and a program; a name is for a person to read, and
  nothing in v2 is read by a person yet. (There is now: §237 built the
  names, and this sentence was written before it did.)

### What it cost

- **Eleven tests** in the MIDI library (`tests/ops.rs`) and **five**
  in the sound battery (`tests/midiedit.rs`): a note is put on the
  nearest line, a note on a line stays, a grid of no ticks does
  nothing, the strength is how far a note moves, a transpose moves the
  pitch and is refused out of the range, the velocity is set, added
  and scaled and is refused out of the range, the length is changed,
  the legato ends a note before the next, the notes are split by their
  pitch — and the notes of a clip are quantised onto the grid,
  transposed and given a velocity, with the same refusals.
- **Three commands** (`sound.clip.midi.quantize`,
  `sound.clip.midi.transpose` and `sound.clip.midi.velocity`). The MCP
  server now carries **279** commands, and the sound battery carries
  **68**.

### What it does not settle

- **The tree's other operations are not here.** The velocity may be
  set, added to and scaled; the length may be set, added to, scaled,
  made legato and made free of overlap; and the tree has a velocity
  that is **limited** to a range, one that **ramps** over the notes
  and one that **humanises** them. The last three are not here, and
  the humanise is a random one — and a random fact is not a fact.
  (The **limit** and the **ramp** are here now: §229 built them, and
  this sentence was written before it did.)
- **The quantise has no strength of its own here.** The library's
  quantise has one — how far a note moves towards the line — and the
  battery's command moves every note all the way. A person who wants
  a note moved half way to the line has not got a command for it yet.

## 220. The healing of a track

The tree's editor heals a separation: it walks the clips of a track in
order and joins each one with the one before it when they are **the
same sound continued**. The cut of §212 is the cut that is undone, and
until now nothing undid it.

### What was built

- The command `sound.clip.heal` heals a track: the clips of the track
  are walked in order and each one is joined with the one before it
  when they are the same sound continued. The answer says how many
  joins were made.
- Four things must hold for two clips to be one sound: the **same
  source**, the **offset lines up** — the second starts in the source
  where the first's sound carries on, the distance of any gap on the
  timeline **divided by the stretch** — the **same way of playing it**,
  and the second must not **start before** the first ends.
- The joined clip is the **first** one, made longer: it keeps its own
  head, its own fade in and its own beat, and it takes the second's
  fade out.
- The second clip's **lanes** move into the first, and their
  breakpoints move with them: a lane that was over the second piece is
  over the second piece of the sound.

### What the port found

- **The tree's rule is about the content and so is this one.** Two
  clips that touch are not a separation: the second must start in the
  source where the first's sound carries on. A clip that was moved
  without its content moving is a different sound, and it stays apart.
- **The tree checks the mute, the stretch and the gain, and this tree
  checks the pitch as well.** The tree's clip has no pitch of its own
  — the pitch is the track's business — and this tree's clip has one
  (§208), and a clip played at another pitch is not the same sound
  continued.
- **The tree's gain envelope is joined and this tree's lanes are
  moved.** The tree keeps a clip's curve in the clip's own record and
  joins two curves into one at the seam; here a curve is a **lane**, a
  node of its own under the clip it automates, so the lanes of the
  second piece are moved into the first and their breakpoints move
  with them.
- **The tree heals a range and this tree heals a track.** The tree's
  command takes a selection, because a person heals the part of a
  track they are looking at. Here the track is the unit: the clips of
  the whole track are walked, and the joins are made where they are.

### What it cost

- **Six tests** in the sound battery (`tests/heal.rs`): a separated
  clip is healed, clips of different sources stay apart, clips that
  overlap stay apart, clips played differently stay apart, the lanes
  of the second piece move into the first, and the fade out of the
  second piece is kept.
- **One command** (`sound.clip.heal`). The MCP server now carries
  **280** commands, and the sound battery carries **69**.

### What it does not settle

- **The tree's beat is not here.** The tree's clip has no beat of its
  own; this tree's clip has one (§196), and the joined clip keeps the
  first piece's beat and drops the second's — unless the first piece
  has none, and then the second's moves with the sound it is in. A
  clip that carried two beats carries one after the healing, and that
  is a small loss of a thing the tree never had to lose.
- **The tree's range is not here.** A person heals the part of a track
  they are looking at, and this tree heals the whole track: there is
  no range in the command, and the whole track is walked.

## 221. The separation of a clip at its transients

The tree's editor separates a take at the **transients** of its sound:
the places where the sound starts again — a drum, a plucked string,
the start of a phrase. The clip was cut at a place (§212) and on the
grid (§215), and the sound itself was never listened to.

### What was built

- The **audio crate** grows a **detector**: `transients` finds the
  onsets of a sound and says where they are, in the sound's own
  samples.
- The command `sound.clip.separate.transients` separates a clip at the
  transients of the **material** it plays — the source — and not in
  the clip's own sound: the clip's sound is the material read the
  clip's way, and the material's own attacks are where the pieces are.
- The **sensitivity** is how small a rise may be and still count, from
  zero to one.
- The number of cuts is **bounded**: a clip with more transients than
  the bound is **refused** and not cut at the first thousand of them.

### What the port found

- **The tree's detector watches the spectrum and this one watches the
  level.** The tree takes a fast Fourier transform of each frame and
  looks for a jump in the high bands; that is a better answer, and it
  needs a transform this kit does not have — the kit's crates carry no
  dependencies at all, and a fast Fourier transform is a piece of work
  of its own. So the detector here reads the **level** of each short
  frame and calls a transient a rise that stays up, and the difference
  is written down where the detector is. (There is now: §244 built the
  transform and the tree's detector, and this sentence was written
  before it did.)
- **The tree's sensitivity and this one's are not the same number.**
  The tree's sensitivity is how much of the frame's own spectrum must
  change; here it is how much of the **whole sound's** level a frame
  must reach to count — a hundredth of it at the most sensitive, and
  all of it at the least. A number outside zero to one is clamped into
  it: the sensitivity is a knob and not a fact about the sound. (The
  detector is the tree's now, and so is the sensitivity: §244.)
- **The tree's transients are of the clip and this one's are of the
  material.** The tree reads the clip's rendered sound, so a clip that
  is stretched or pitched is heard as it will be played; here the
  source is read, and the clip's stretch is undone on the way back —
  because a stretched clip reads its material more slowly, and the
  piece is where the material's attack was.

### What it cost

- **Six tests** in the audio crate (`tests/transients.rs`): the hits of
  a sound are found, a fade has no transients, silence has no
  transients, the sensitivity is how small a rise may be, two hits that
  are too close are one hit, and a stereo sound is read across both of
  its channels.
- **Three tests** in the sound battery (`tests/transients.rs`): a clip
  is separated at its transients, a clip that plays no sound is
  refused, and the sensitivity is how small a rise may be.
- **One command** (`sound.clip.separate.transients`). The MCP server
  now carries **281** commands, and the sound battery carries **70**.

### What it does not settle

- **The tree's detector is better and this one is not.** A spectral
  detector hears a drum inside a chord, and a level detector hears the
  chord. A fast Fourier transform is a piece of work of its own, and
  it is not done: the kit's crates carry no dependencies at all, and
  that is a rule worth more than a better detector. (There is now:
  §244 built the transform — in the kit's own crates, with no
  dependency at all — and the detector is the tree's.)
- **The tree reads the clip and this one reads the source.** A clip
  whose sound is changed on the way out — a process, a pitch — is
  heard by the tree as it will be played, and here the material is
  heard as it is on the disk.
- **The silence is not cut out.** The tree's strip silence removes the
  quiet parts of a clip and names the pieces that are left; here the
  transients are found and the clip is cut at them, and the quiet parts
  are still in the pieces.

## 222. The stripping of silence

The tree's editor strips silence: it finds the parts of a sound that
are not silent, cuts the clip at the edges of them, and keeps the loud
parts. The clip was cut at a place (§212), on the grid (§215) and at
its transients (§221), and the quiet parts of it were still in it.

### What was built

- The **audio crate** grows `loud_ranges`: the parts of a sound that
  are loud enough to keep, in the sound's own samples.
- The command `sound.clip.strip.silence` strips the silence out of a
  clip: the clip is cut at the edges of the loud parts, the quiet
  pieces are thrown away, and the loud ones are kept.
- The **threshold** is how far below the loudest part of the material
  a part may be and still be kept, in decibels; the **minimum length**
  is how long a part must be, and a click is not a part; the **pad** is
  how much silence is kept at each end of a part, so that a part does
  not begin or end with a click.
- The kept pieces are **named** from the auto rename scheme (§217), in
  time order.

### What the port found

- **The tree's pieces are named and the thrown-away ones are not.** The
  tree names the pieces it keeps, in time order, and the pieces it
  removes take no name. Here the cut names every piece it makes — the
  cut is the cut of §212 and it does what it does — so the scheme's
  number is **put back** before the kept pieces are named: the kept
  pieces take the numbers of the scheme and nothing is skipped.
- **The tree's threshold is a level and this one's is too.** The
  tree's threshold is forty-eight decibels by default and it is
  measured against the loudest part of the sound; here it is the same
  number and the same meaning, and a number outside minus ninety-six
  to zero is clamped into it — the threshold is a knob and not a fact
  about the sound.
- **The tree works on a track and a range, and this one works on a
  clip.** The tree's command takes a track and a selection, because a
  person strips the part of a track they are looking at. Here the clip
  is the unit, and the clip is cut and the quiet pieces of it are
  thrown away.
- **The tree's silence is of the clip and this one's is of the
  material.** The tree reads the clip's rendered sound, so a clip that
  is stretched or processed is heard as it will be played; here the
  source is read, and the clip's stretch is undone on the way back.

### What it cost

- **Four tests** in the sound battery (`tests/strip.rs`): the silence
  is stripped out, the kept parts are named from the scheme, a silent
  clip is refused, and a clip that plays no sound is refused.
- **One command** (`sound.clip.strip.silence`). The MCP server now
  carries **282** commands, and the sound battery carries **71**.

### What it does not settle

- **The tree's three modes are not here.** The tree's command strips
  the silence, **extracts** it, or separates at every edge and keeps
  everything; here the silence is stripped, and the other two are not.
  (All three are here now: §231 built the other two, and this
  sentence was written before it did.)
- **The tree's preview is not here.** The tree scans a track without
  changing it, so that a person can see the ranges before they strip
  them; here the command strips, and there is no scan to see. (The
  scan is here now: §231 built it as a read, and this sentence was
  written before it did.)

## 223. The editing of the notes of a MIDI clip

The tree's MIDI editor has a pencil: a note is added, edited, taken
away and moved. The notes of a MIDI clip could be read and written
(§218) and quantised, transposed and given a velocity (§219), and
**not one of them could be changed**: the notes were a fact with no
command to touch them.

### What was built

- The command `sound.clip.midi.add` makes a **MIDI clip with no notes**
  in it, a bar long at the document's tempo unless the caller says
  how long it is.
- The commands `sound.clip.midi.note.add`, `sound.clip.midi.note.edit`,
  `sound.clip.midi.note.remove` and `sound.clip.midi.notes.move` add,
  change, take away and move the notes of a clip.
- A note's **pitch**, **velocity** and **channel** are refused when
  they are outside the range a MIDI note has, and an **index** that
  names no note is refused.
- The **room** of a MIDI clip: the length is the room the notes need
  and never less than the room the clip has.

### What the port found

- **The tree clamps and this tree refuses.** The tree's pencil clamps
  the pitch, the velocity and the channel into the range a MIDI note
  has, and a note asked for as a hundred and forty is a note the
  caller does not know about. Here every one of them is refused and
  the note is not made — the same rule the transpose follows (§219).
- **The tree's clip has a length of its own and the notes do not touch
  it.** Here the notes may make the clip **longer** — a note that
  reaches beyond the clip makes room for itself — and they do not make
  it shorter: the room is the room, and a note that moves away does not
  take the room with it.
- **This is a change, and §219 said the other thing.** §219 said the
  length of a MIDI clip was "worked out again from the notes" when the
  notes were quantised, and that was true when it was written: the
  length was the length of the notes. It is not true now, and the
  reason is the tree's own: the tree's clip has a length and the
  sequence does not touch it. So the length is the **greater** of the
  room the clip has and the room its notes need, and §219's text has
  been corrected where it stood.
- **The tree's new MIDI clip is on an instrument track and this one's
  is not.** The tree makes a MIDI clip on a track that plays an
  instrument, and there is no instrument in this battery — a MIDI clip
  is silent (§218) — so the clip is made on any track and it holds
  notes and no sound.

### What it cost

- **Nine tests** in the sound battery (`tests/midinotes.rs`): a MIDI
  clip is made with no notes, a note is added, a note is edited, a
  note is removed, notes are moved, a note outside the range is
  refused, a move out of the range is refused, an index that names no
  note is refused, and a clip that is not a MIDI clip is refused.
- **Five commands** (`sound.clip.midi.add`, `sound.clip.midi.note.add`,
  `sound.clip.midi.note.edit`, `sound.clip.midi.note.remove` and
  `sound.clip.midi.notes.move`). The MCP server now carries **287**
  commands, and the sound battery carries **76**.

### What it does not settle

- **The tree's controllers are not edited.** A MIDI clip's controllers
  are read and written (§218) and there is no pencil for them: a
  controller may be added, changed or removed by no command.
- **The tree's duplicates are not taken out.** The tree has a command
  that takes the duplicate notes out of a clip, and the library here
  can do it (§ the MIDI library's `remove_duplicates`) — but there is
  no command that asks for it. (There is now: §239 built it, and this
  sentence was written before it did.)
- **The tree's note names are not here.** A note is a pitch, a place
  and a length, and it has no name: the tree names a note "C3", and a
  name is for a person to read. (There is now: §237 built the names,
  and this sentence was written before it did.)

## 224. The taking away of a link

A route between a track and a bus, a track's reading of a bus and a
send's link to a bus are **relations** and not nodes — and a relation
is not a node that `node.delete` can take away. So a routing mistake
in v2 was permanent: the link could be made and never unmade.

### What was built

- The command `sound.route.remove` takes away the **route**: the track
  stops feeding the bus.
- The command `sound.track.input.remove` takes away the **reading** of
  a bus by a track.
- The command `sound.send.remove` takes away a **send**, and its link
  goes with it.
- The command `sound.source.remove` takes away a **source**, and the
  **asset** it brought goes with it: a document does not need a file
  that nothing plays.
- A link that is **not there** is refused, and it is refused by name:
  a link that was never made is not a link that was taken away.

### What the port found

- **The tree's mixer removes these things and v2 could not.** The tree
  has `mix.insert_remove` and `mix.send_remove`, and its routes are a
  property of a track — the tree's track has one output, and setting
  it again is how a route is changed. Here a track may feed **several**
  buses, so a route is a list of edges and the taking away of one is a
  command of its own.
- **A relation is not a node, and the kit's own `node.delete` cannot
  reach it.** That is the finding of this arc: the sound battery's
  links were the one thing in the document that could not be undone,
  and a document that cannot undo a mistake is a document that keeps
  it. The sound battery now has commands of its own for the links, and
  they are the same commands the tree has for the same things.
- **The tree's source is a file the session names and the session does
  not lose.** Here a source is a node **and** an asset — the document
  says it needs the file — so taking the source away takes the asset
  away with it. A document that keeps a file nothing plays is carrying
  a file for nothing.

### What it cost

- **Six tests** in the sound battery (`tests/links.rs`): a route is
  taken away, a route that is not there is refused, a reading is taken
  away, a send is taken away, a source is taken away with its asset,
  and a node that is not a send is refused.
- **Four commands** (`sound.route.remove`, `sound.track.input.remove`,
  `sound.send.remove` and `sound.source.remove`). The MCP server now
  carries **291** commands, and the sound battery carries **80**.

### What it does not settle

- **The tree's inserts are not removed by a command of their own.**
  They are nodes, and `node.delete` takes them away with their links;
  the tree has `mix.insert_remove`, and here the insert is a node
  like any other.
- **A clip's link to its source is not taken away.** A clip that plays
  a source says so in its own payload, and clearing that is a change
  to the clip and not to a link; the tree's clip has a closed kind of
  content and a clip without a source is not a clip there.

## 225. The editing of a lane

The tree has five commands for a lane: write a value over a range,
clear the breakpoints, thin them, and write the current value over the
range. v2 had **one** — a lane could be made with its points and not
changed afterwards. So a curve could be drawn and not drawn again, and
a mistake in it could not be undone.

### What was built

- The commands `sound.lane.point.set` and `sound.lane.point.remove`
  set a **breakpoint** and take one off.
- The command `sound.lane.clear` clears the breakpoints of a lane: all
  of them, or the ones in a range.
- The command `sound.lane.write` writes a **value over a range**: the
  lane is flat over it and the curve outside it is kept, with anchors
  just before and just after.
- The command `sound.lane.thin` takes away the breakpoints **the curve
  does not need**: a breakpoint goes when the curve through the ones
  that remain changes by less than the tolerance.
- A **pan** outside minus one and one, and a **mute** outside zero and
  one, are refused.

### What the port found

- **The tree's lane is a curve and this tree's lane is a curve too.**
  The tree's lane holds its points in the track's record; here a lane
  is a **node** with its points in its payload, and a lane is
  automated by a track or a clip (§ the automation). The editing is
  the same either way, and the commands are the tree's.
- **The tree's thinning is a slope, and so is this one.** A breakpoint
  goes when the slope from the last breakpoint that was kept can reach
  the next one without leaving the tolerance — the same curve without
  it. The first and the last breakpoints always stay: a curve needs
  its ends.
- **The tree clamps the value and this tree refuses.** The tree's
  `set_point` clamps the value into the range of the parameter — the
  pan between minus one and one, the mute between zero and one — and a
  value the caller did not ask for is a value the caller does not know
  about. Here the value is refused, and the lane keeps what it had.
- **The tree's commands work on a track and a selection; this tree's
  work on a lane.** The tree's lane belongs to a track and its
  commands take the tracks and the selection; here a lane is a node,
  and the command names the lane — because a lane may automate a clip
  as well as a track, and the tree's clip carries a curve of its own.

### What it cost

- **Eight tests** in the sound battery (`tests/lanes.rs`): a
  breakpoint is set, a breakpoint that is there takes the new value, a
  breakpoint is taken off, a lane is cleared, a value is written over a
  range, a lane is thinned, a value outside the range is refused, and a
  node that is not a lane is refused.
- **Five commands** (`sound.lane.point.set`, `sound.lane.point.remove`,
  `sound.lane.clear`, `sound.lane.write` and `sound.lane.thin`). The
  MCP server now carries **296** commands, and the sound battery
  carries **85**.

### What it does not settle

- **The tree's "write to current" is not here.** The tree can write
  the value a track already has over a range — the value of the fader,
  the pan, the mute — and here the value is given by the caller. The
  value of a track's fader is the fader's own field (§ the level), and
  a command that reads it and writes it over a range is a command that
  has to know which value the caller meant. (It is here now: §234
  built it, and this sentence was written before it did.)
- **The tree's thin touches every lane of a track at once.** Here a
  lane is thinned on its own: the command names the lane, and a person
  who wants every lane thinned asks for them one after another. (All
  the lanes at once are here now: §234 built it, and this sentence was
  written before it did.)

## 226. The moving of a clip

The tree's editor nudges the clips and shifts the ones on a track. v2
had neither: a clip was **placed** and **trimmed** and never moved,
because the trim changes what is heard and a move changes when it is
heard — and there was no command for the second thing.

### What was built

- The command `sound.clip.move` moves clips by so many samples, later
  or earlier, and their **content** goes with them: a clip that moves
  keeps the sound it plays and plays it at another time.
- A **locked** clip does not move — the edit lock and the time lock
  both hold it — and the answer says how many moved.
- A move that would take a clip **before the beginning** of the
  timeline is refused, and no clip moves: a move that is half made is
  a move nobody asked for.
- The command `sound.clip.trim.to.file` trims a clip to the **end of
  its material**: a clip that reaches past the end of its source plays
  silence there, and a clip that is half silence says something false
  about its own sound.

### What the port found

- **The tree's locked clip "doesn't move", and this tree's did not
  move at all.** The tree's own word for it is in the tree's editor:
  a clip that is time-locked **or** edit-locked does not move. v2's
  locks module said as much in its header and then said the opposite
  in the edit lock's own words — that a locked clip "may still be
  moved" — and the second sentence was wrong. It is corrected now, in
  the place where it stood, because a command that moves a clip has to
  know what a lock means.
- **The tree's trim to the file's boundaries stops at the
  neighbours.** The tree's command trims a clip to the material it
  plays **and** stops at the clip before and the clip after it, so
  that a trim never overwrites. Here a clip may overlap another —
  a crossfade is an overlap — and so the trim stops at the material's
  end and at nothing else.
- **The tree's move overwrites what it lands on.** The tree's editor
  moves the clips by a delta and the clips that were there go. Here a
  clip that lands on another is an **overlap** and both are heard:
  the mix sums them, and a crossfade is two clips that share a time.

### What it cost

- **Five tests** in the sound battery (`tests/move.rs`): a clip is
  moved, a locked clip does not move, a move before the beginning is
  refused, a clip is trimmed to its material, and a clip inside its
  material is not trimmed.
- **Two commands** (`sound.clip.move` and `sound.clip.trim.to.file`).
  The MCP server now carries **298** commands, and the sound battery
  carries **87**.

### What it does not settle

- **The tree's shift is not here.** The tree shifts every clip of a
  track from a place onwards, and here the clips are moved one
  command at a time: the command takes the clips it moves, and a
  person who wants a whole track shifted asks for its clips. (Every
  clip from a place onwards is shifted now: §236 built it as the
  insertion of silence, and this sentence was written before it did.
  What is still not here is a shift by an **arbitrary** amount — the
  tree's shift moves the clips by any distance it is given, and
  v2's shifts them by the length of the silence inserted.)
- **The tree's trim to fill the selection is not here.** It trims the
  clips to the edges of a selection, and here the trim is to a range
  the caller gives or to the material the clip plays.

## 227. The controllers of a MIDI clip

The tree's MIDI editor has a pencil for the **notes** and none for the
**controllers** — the volume, the pan, the pitch bend, the program, the
pressure. A MIDI clip could hold them, read them and write them, and
not one of them could be changed.

### What was built

- The commands `sound.clip.midi.ctrl.add`, `sound.clip.midi.ctrl.edit`
  and `sound.clip.midi.ctrl.remove` add, change and take away the
  **controllers** of a clip.
- The command `sound.clip.midi.notes.dedupe` takes the **duplicate
  notes** out of a clip.
- The **kind** of a controller says what it changes: a `control`
  change, a `pitch_bend`, a `channel_pressure`, a `poly_pressure` or a
  `program`. A kind that is not one of them is refused, and so is a
  number for a kind that has none and a missing number for a kind that
  has one.
- The **value** is in the range of its kind: a control change, a
  pressure and a program are from zero to a hundred and twenty-seven,
  and the pitch bend is centred on zero.

### What the port found

- **The tree's editor has no pencil for the controllers, and this tree
  does.** That is the finding of this arc: the tree's MIDI editor
  moves notes about and leaves the controllers to the file they came
  from, and here the controllers of a clip are editable — which is
  *more* than the tree has, and the honest way to say so is to say it.
- **The kinds are open strings, and the range of each is checked
  against its kind.** A control change and a program are numbers from
  zero to a hundred and twenty-seven and the pitch bend is a signed
  number about eight thousand — so a value is refused when it is
  outside the range of **its** kind, and not against one range for
  them all.
- **The tree's duplicates are taken out by a command and so are
  theirs here.** The tree's `midi.remove_duplicate_notes` and the
  library's `remove_duplicates` are the same thing: two notes of the
  same channel, pitch and start are one note, and the longer one is
  kept; two controllers of the same kind, channel and tick are one
  controller, and the last one is kept.

### What it cost

- **Five tests** in the sound battery (`tests/midictrl.rs`): a
  controller is added, a controller is edited, a controller is
  removed, a controller outside its range is refused, and the duplicate
  notes are taken out.
- **Four commands** (`sound.clip.midi.ctrl.add`,
  `sound.clip.midi.ctrl.edit`, `sound.clip.midi.ctrl.remove` and
  `sound.clip.midi.notes.dedupe`). The MCP server now carries **302**
  commands, and the sound battery carries **91**.

### What it does not settle

- **A controller has no name.** The tree names a drum and a program;
  here a controller is a kind, a number and a value, and the name of
  the thing it changes is a name for a person to read.
- **The controllers of a clip are not moved with the notes.** A cut
  moves the notes of a clip (§212) and the controllers with them — the
  tree's cut splits the sequence, notes and controllers together — and
  the *move* of a controller is the command that edits it.

## 228. The events of the session

A marker could be added and not moved, and the tempo map could be
given a tempo and not scaled. The tree edits both — its markers have
an edit command, and its tempo map has "Scale..." and "Constant..." —
and v2 could only put things there.

### What was built

- The command `sound.marker.set` moves a marker or **renames** it, and
  the marker moves as a **whole**: its span is kept.
- The command `sound.tempo.scale` multiplies every tempo by a
  **factor**: the music is faster everywhere and the times the tempos
  happen at do not move.
- The command `sound.tempo.constant` makes a **range** of the tempo
  map constant: the tempo at the start of the range is the one given,
  the tempo changes inside it are taken away, and the tempo at the end
  of it comes back.
- A factor that would take a tempo **out of the range** a tempo may
  have is refused, and no tempo is scaled.

### What the port found

- **The tree's marker is edited and this tree's marker was not.** The
  tree's `markers.edit` moves a marker and renames it; here a marker
  was a node that could be made and deleted and not touched — and a
  marker is one of the few things in a sound document that a person
  names.
- **The tree's scale multiplies the tempos and keeps the times.** The
  tree's `event.tempo_scale` is the same operation: a factor of two
  is twice as fast everywhere and the music still starts at the same
  moment. The times are **ticks** and they do not move, because a
  tempo is how fast the music goes and not when it goes.
- **The tree's constant range reads the end before the edit.** The
  tree's `event.tempo_constant` reads the tempo at the end of the
  range **before** it changes the tempo at the start of it — because
  the end is a musical place and the new tempo moves it. Here the
  same: the tempo at the end of the range is read first and put back
  afterwards, so that the music after the range is as fast as it was.
- **The tree's tempos are in its session and these are in the
  document.** The tree's tempo map is a value of the session; here the
  tempo events are **nodes** — the same rule as the markers and the
  meter — and the map is worked out from them every time (the tempo
  map is derived, §19).

### What it cost

- **Four tests** in the sound battery (`tests/events.rs`): a marker is
  changed, the tempos are scaled, a factor out of the range is
  refused, and a range of the tempo map is made constant.
- **Three commands** (`sound.marker.set`, `sound.tempo.scale` and
  `sound.tempo.constant`). The MCP server now carries **305**
  commands, and the sound battery carries **94**.

### What it does not settle

- **The tree's insert and cut of time are not here.** The tree can
  insert time at a place or cut it out, and the events and the clips
  after it move: that is a change to the whole of the session's
  timeline, and it is not a command here. (This was written before
  §236, which builds both commands; the sentence is left as it was
  and the arc that made it true is beside it.)
- **The tree's renumbering of the bars is not here.** The tree's
  session counts its bars; here the grid counts from the bar line
  (§200) and there are no bar numbers to renumber.

## 229. The velocity of a note

The tree changes how hard a note is struck in four ways: it sets the
velocity, adds to it, scales it and **limits** it. v2's command had
only the first two — the library had the **scale**, and nothing had
the **limit** or the **ramp**.

### What was built

- The command `sound.clip.midi.velocity` takes a `scale` beside its
  `set` and its `delta`, and a **limit** as its `min` and its `max`.
- The library's velocity grows two ways: a **limit**, which moves a
  note onto the edge of the limit, and a **ramp**, which strikes the
  notes harder and harder along their span — the earliest note at the
  first value, the latest at the second.
- The limit's bounds are **swapped** when they are the wrong way
  round, so a limit of a hundred and ten is a limit of ten and a
  hundred.
- The limit's own bounds are **refused** when they are outside the
  range, and a limit with only one bound is refused as well: a limit
  says no softer than this **and** no harder than that.
- The velocity is changed **one way at a time**: two of the four at
  once is refused, because a caller that meant one and got the other
  would not know.

### What the port found

- **The tree's limit is the one velocity change that clamps on
  purpose.** Every other velocity the tree changes is clamped into the
  range — a note asked for as a hundred and fifty is struck as a
  hundred and twenty-seven — and this tree **refuses** all of those
  (§219). But a **limit** says *no softer than this and no harder than
  that*, so moving a note onto the edge of the limit is what was
  asked for and not a quiet correction. So the limit clamps and
  everything else refuses, and the limit's own bounds are still
  refused: a limit of zero is not a limit of anything, because a
  velocity of zero is the way a note is let go.
- **The tree's humanise is random and this one is not.** The tree's
  velocity has a `Humanize` of its own, which moves every note by a
  random amount. It is not here, and it is not here for the same
  reason the tree's random quantise is not (§219): a note struck
  harder by a different amount each time is not a note the caller
  knows about. The tree's **limit** and **ramp** are here, because both
  of them say the same thing every time they are asked.
- **The tree's command takes the ways in an order, and this one takes
  them one at a time.** The tree's `event.velocity` tries `set`, then
  `add`, then `scale`, and a limit is whatever is left over — so a
  caller that gave both a `set` and a `scale` got the `set` and never
  heard about the `scale`. Here two ways at once are **refused** and
  the caller is told which ways there are, because a command that
  silently drops half of what it was asked for is a command that
  answers a question nobody asked.

### What it cost

- **Nine tests**: five in the MIDI library (`tests/ops.rs` — the
  limit, the swapped bounds, a limit out of range, the ramp and a ramp
  of one note) and four in the sound battery (`tests/velocity.rs` —
  the scale, the limit, a limit with one bound and two ways at once).
- **One command** changed and **one library** grown: the sound battery
  still carries **94** commands and the MCP server **305**, because a
  command that takes more parameters is not another command.

### What it does not settle

- **The tree's humanise is not here**, and that is a decision and not
  an oversight (§219): the tree's version moves every note by a random
  amount, and this tree does not do random. A caller that wants a
  human feel can read the velocities and change them itself.
- **The tree's velocity is a command over a selection, and this one is
  a command over a clip.** The tree's `event.velocity` works
  on every clip that is selected, and a selection is a thing of the
  editor's state — which this tree does not keep (§200). Here the
  clip is named, and a caller with several clips says so several
  times.

## 230. The length of a note

The tree changes how long a note lasts in four ways: it sets the
length, adds to it, scales it and makes the notes **legato**. v2's
library had all four operations and no command reached them — the
notes of a MIDI clip could be struck at another velocity but not made
another length.

### What was built

- The command `sound.clip.midi.duration` takes a `set`, a `delta`, a
  `scale` and a `gap` — the gap is the **legato**: every note ends so
  many ticks before the next note starts, and the last note keeps its
  length.
- A note is never shorter than **one tick**: a note of no length is a
  note that cannot be heard.
- The length is changed **one way at a time**, as the velocity is
  (§229).
- The **room** of the clip is worked out again from the notes: the
  room grows when the notes need more of it, and it does not shrink
  when they need less.

### What the port found

- **The tree's command is over a selection and this one is over a
  clip.** The tree's `event.duration` works on every clip that is
  selected, and a selection is a thing of the editor's state, which
  this tree does not keep (§200). Here the clip is named.
- **The tree's length is clamped and this one is a shape.** The tree's
  command makes a note's length `max(1)` — a note of no length is
  never made — and that is a **shape** and not a bound: a note that
  cannot be heard is not a note at any length, so there is no length
  that could be asked for and refused. Here the same rule holds, and
  the shape lives in the library, where the operation is.
- **The tree's scale is clamped and this one is refused.** The tree
  takes the factor a caller gave and pushes it into the range between
  a hundredth and a hundred, so a factor of zero comes out as a
  hundredth and the caller never hears about it. Here a factor that is
  not a number, or is negative, is refused and the notes are not
  changed (§219, the same rule the transpose follows).

### What it cost

- **Five tests** in the sound battery (`tests/duration.rs`): the
  length is set, changed and scaled; a note is never shorter than a
  tick; the notes are made legato; two ways at once are refused; and
  the room grows and does not shrink.
- **One command** (`sound.clip.midi.duration`). The sound battery now
  carries **95** commands and the MCP server **306**.

### What it does not settle

- **The tree's `RemoveOverlap` is not here.** The tree's library has
  an operation that shortens a note so that it ends no later than the
  next note of the same pitch and channel starts, and its own command
  does not reach it — so here it is in the library and not in a
  command, for the same reason (§219: a field with no command is not
  a fact).
- **The tree's length change is over a selection**, and here it is
  over a clip: the same gap as the velocity's (§229).

## 231. The modes of the strip of silence

The tree's editor strips silence in three ways: it **strips** the
silence and keeps the loud parts, it **extracts** the sound and keeps
the silence, and it **separates** the clip at the edges of the loud
parts and keeps everything. It also has a **preview** that finds the
silence and says where the parts are without changing anything. v2 had
the strip alone.

### What was built

- The command `sound.clip.strip.silence` takes a **mode**: `strip`
  keeps the loud parts and throws the quiet ones away, `extract` keeps
  the quiet parts and throws the loud ones away, and `separate` cuts
  the clip at the edges of the loud parts and keeps everything.
- A mode this battery does not know is **refused**, and the clip is
  not changed.
- The command `sound.clip.strip.silence.read` is the **preview**: it
  says where the loud parts are and it changes nothing.
- The loud parts are worked out by **one** helper, which the strip and
  the preview both call — so the preview cannot say something the
  strip would not do.

### What the port found

- **The tree's mode is a string and this one is a string too.** The
  tree's `StripMode` is a variant of three, matched in the engine; here
  the mode is a **name** in a parameter, and a name this battery does
  not know is **refused** — the same rule the **processes** on a clip
  follow (§209), and the same reason: a mode that does nothing is
  worse than no mode at all.
- **The tree's preview is a scan that runs on a thread, and this one
  is a read.** The tree's `edit.strip_silence_preview` builds a scan
  and runs it, on a worker thread in the desktop build and on the frame
  thread in the web build, because the scan renders the audio and that
  is work a frame cannot wait for. Here a command runs on the caller's
  thread, and a read that renders a file is work the caller asked for
  (§212, the same rule the peaks follow). The **shape** is the same:
  the preview finds the silence and says where the parts are, and it
  changes nothing.
- **The tree's strip works on a selection and this one on a clip.** The
  tree's `edit.strip_silence` takes a **range** and a list of tracks,
  and it finds the silence in the whole of it; here the unit is a
  **clip**, and the parts are found in the **material** the clip plays
  (§222). The tree's separate mode cuts the clip at the edges of the
  loud parts too — and so does this one, at the edges of the parts of
  the clip.

### What it cost

- **Four tests** in the sound battery (`tests/stripmode.rs`): the
  loud parts are extracted, the clip is separated at the edges of the
  loud parts, an unknown mode is refused, and the preview says where
  the parts are without changing anything.
- **One command** (`sound.clip.strip.silence.read`) and **one command
  changed** (`sound.clip.strip.silence`, which takes a mode). The sound
  battery now carries **96** commands and the MCP server **307**.

### What it does not settle

- **The tree's strip of silence is over a selection and a list of
  tracks.** Here it is over a clip: the same gap as the velocity's
  (§229) and the length's (§230). A clip that is one piece of a
  selection is stripped as a clip, and the parts of the selection that
  are in other clips are not looked at.
- **The tree's naming of the clips left in the selection is not here.**
  The tree's strip can name the audio clips that were left inside the
  selection with a scheme of its own — a name, a number, a count of
  zeros and a suffix — beside the auto rename scheme. Here the
  **auto rename** scheme names the pieces (§216), and the strip's own
  naming is not a command: a scheme is a node here (§216), and a
  second way of making one would be a second path.

## 232. The cut of a MIDI clip

Separating a **MIDI clip** kept **all** of its notes in **both**
pieces. The tree divides the notes at the cut: the notes before it
stay in the first piece and the notes at or after it move to the
second. This tree did not — and §212, which describes the separation,
did not say so.

### What was built

- The cut **divides the notes**: the notes before the cut stay in the
  first piece, and the notes at or after it move to the second.
- The second piece's notes are **counted from its own head**: a note
  that was a quarter note into the clip and a quarter note after the
  cut is at the head of the second piece.
- The **controllers** are divided the same way as the notes, for the
  same reason.
- The fix is in the **one** place the cut is made, so the separation
  on the **grid** and at the **transients** divide the notes too.

### What the port found

- **The tree's cut divides the notes and this tree's did not.** The
  tree's `split_clip` keeps the notes before the cut in the left
  piece and the notes at or after it in the right one, and moves the
  right one's notes back by the cut. Here the payload was **cloned**,
  so both pieces had every note: a clip of two notes became two clips
  of two notes, and the notes were heard **twice**.
- **The tree's cut is a cut and this tree's was a copy.** The tree's
  `split_clip` is one function that trims the clip and trims a copy;
  here the second piece was built by cloning the payload and moving
  the fields that a cut moves — the length, the fade, the beat, the
  offset. The **notes** were not in that list, because a clip of sound
  has no notes and the cut was written before a MIDI clip existed
  (§218). The lesson is the one the kit already teaches: a cut is one
  operation, and every field that moves in a cut has to be in it.
- **The notes are in ticks and the cut is in samples.** The tree's
  `split_clip` asks the **tempo map** for the boundary in ticks. Here
  the same: the boundary is the distance the cut made, in the ticks of
  the document's tempo at the clip's own start — so the division
  follows the tempo, and a clip that is cut on a beat divides on that
  beat.

### What it cost

- **Three tests** in the sound battery (`tests/midiseparate.rs`): the
  cut divides the notes, the second piece counts its notes from its
  own head, and a note is not in both pieces.
- **No command**: the separation was already there, and it was wrong.
  The sound battery still carries **96** commands and the MCP server
  **307**.

### What it does not settle

- **The tree's separate is over a track and this one is over a
  clip** (§212), and that is unchanged: the fix is in the cut, not in
  what the cut is given.
- **A note that straddles the cut is not split.** The tree's cut puts
  a note that is sounding across the cut in the piece that holds its
  **start**, and the note is not shortened; here the same, because the
  division is by the note's **start** and not by the note's end. A
  note that begins before the cut and ends after it is in the first
  piece and is heard past the cut's end.

## 233. The trim of a MIDI clip

The **trim** of a MIDI clip left its **notes** where they were. The
clip's head moved, its content moved and its beat moved — and the notes
stayed, so a clip trimmed a quarter note in still played the note that
was at its head. The tree's trim **rebases** the notes: the notes the
trim took off are gone and the notes that are left are counted from the
clip's new head.

### What was built

- The trim of a MIDI clip takes the **notes off the head**: the notes
  before the trim are dropped, and the notes after it are counted from
  the clip's new head.
- The **controllers** are rebased the same way as the notes.
- The boundary is the distance the trim made, in the **ticks** of the
  document's tempo at the clip's own start — so the rebase follows the
  tempo, and a clip trimmed on a beat rebases on that beat.
- The trim of the **tail** leaves the notes alone, which is what the
  tree does: a shorter clip has the notes it had.

### What the port found

- **The tree rebases the notes and this tree did not.** The tree's
  `trim_start_to` moves the clip's content and its beat, and the
  **engine** re-anchors the MIDI notes after it — the model's own trim
  cannot, because the notes are in ticks and the model does not know
  the tempo map. Here the same: the trim moves the content and the
  beat, and the notes are rebased against the tempo map.
- **A trim is one operation and every field that moves in it has to be
  in it.** This is the same lesson as the cut (§232), found the second
  time: the trim was written before a MIDI clip existed (§218), and
  the notes were not in it. The offset moved, the beat moved and the
  notes did not — because a clip of sound has no notes. A clip of
  notes is a different clip, and the fields that hold its content are
  not the fields that hold a sound's.
- **The tree's tail trim does not rebase and neither does this one.**
  The tree's `trim_end_to` shortens the clip and leaves the notes
  alone: a note that starts before the trim and sounds past it is
  **not** shortened, because a trim of the tail does not touch the
  notes. Here the same.

### What it cost

- **Two tests** in the sound battery (`tests/miditrim.rs`): the trim
  takes the notes off the head, and the trim of the tail leaves them
  alone.
- **No command**: the trim was already there, and it was wrong. The
  sound battery still carries **96** commands and the MCP server
  **307**.

### What it does not settle

- **The tree's trim is over a selection and this one is over a clip**
  (§212): the same gap as the velocity's (§229) and the length's
  (§230).
- **A note that sounds past the trim is not shortened.** The tree
  leaves it whole, and so does this: a note is a note, and cutting it
  in half is a thing the tree does not do on a trim.

## 234. The automation of a whole track

The tree's editor has a **"Thin All"** and a **"Write to Current"**:
the first thins every lane of every track and the second writes the
static value of every parameter over a range. v2 could thin one lane
and write one lane, and not the lot.

### What was built

- The command `sound.lane.thin.all` thins **every lane** of a track:
  the breakpoints the curves do not need are taken away from all of
  them.
- The command `sound.lane.write.all` writes the **static value** of
  every lane of a track over a range: the fader over the volume lane,
  the pan over the pan lane and the mute over the mute lane, so that
  the curve is flat over the range and the value before and after it
  is kept.
- A track with **no** lanes is not a failure: there is nothing to
  write and nothing to thin, and that is said and not thrown.

### What the port found

- **The tree's "Write to Current" writes the static value, and not the
  value at the playhead.** The name is a trap: "current" sounds like
  "where the playhead is", and the tree's `current_value` is the
  track's **static** value — the fader, the pan, the mute — the value
  the parameter has when nothing automates it. Here those values are
  the **defaults of their lanes** (§ the fader), so the command writes
  the default over the range.
- **The tree's mixer keeps a mute of its own beside the mute lane, and
  here the lane is the mute's default.** The tree's track has a
  `mute` in its mixer and a lane that automates the mute, and the two
  are separate: the mixer value is what the track is when no lane
  says else. Here there is one fact — the lane is the mute's default,
  and a lane without breakpoints is the mute — and the write puts the
  track's own value back over the range.
- **The tree's automation is over every track and this tree's is over
  one.** The tree's commands take a list of tracks, because the tree's
  editor has a selection and a selection is a list of things. Here a
  track is a **node** and a node is something a command can be given,
  and a caller with several tracks says so several times. This is the
  same gap as the velocity's (§229) and the length's (§230), and it
  has the same answer.
- **The tree's lane is one of a track's parameters and here a lane is
  a node.** The tree's track holds a list of lanes, one per parameter,
  and the parameters are a closed set — volume, pan, mute, the sends
  and the plugins. Here a lane is a node under a track and its
  parameter is a **string**, so a third party may automate something
  the kit does not know. A lane whose parameter the kit does not know
  has no static value, and the write **leaves it alone** rather than
  writing a number into it that means nothing.

### What it cost

- **Four tests** in the sound battery (`tests/lanesall.rs`): every
  lane of the track is thinned, the static value is written over a
  range, a track with no lanes is nothing to do, and a node that is
  not a track is refused.
- **Two commands** (`sound.lane.thin.all` and `sound.lane.write.all`).
  The sound battery now carries **98** commands and the MCP server
  **309**.

### What it does not settle

- **The tree's commands are over a selection**, and here they are over
  a track: the same gap as the velocity's (§229).
- **A lane of an unknown parameter is left alone.** The kit does not
  know what such a lane's static value is, and writing a number into
  it would be a number that means nothing. A third party that automates
  something of its own may write its own values with its own command.

## 235. The fade of a clip to a place

The tree's editor fades a clip to the **playhead**: "Fade to Start"
fades the clip from its head up to the playhead, and "Fade to End"
fades it from the playhead down to its tail. v2 could give a clip a
fade of a length, and not a fade to a place.

### What was built

- The command `sound.clip.fade.to` fades a clip **up to a place** or
  **down from it**: the fade in runs from the clip's head to the place,
  and the fade out runs from the place to the clip's tail.
- The shape is the tree's own — the **equal-power** curve — and the
  bend is nothing.
- A place that is **not inside the clip** is refused: a fade that ends
  outside the clip is a fade of nothing.
- A way of fading that this battery does not know is **refused**: a
  fade goes `in` or `out`.

### What the port found

- **The tree's fade is to the playhead and this tree's is to a place.**
  The tree's command takes the playhead's position from the editor's
  state, and the editor's state is a thing this tree does not keep
  (§200). Here the place is **given**: a place is a fact, and the
  playhead is a **node** (§ the ruler) — a node that a caller may read
  and pass on, but not a thing every command may reach into.
- **The tree's fade is over a selection and this one is over a clip.**
  The tree's command finds the clip under the playhead on each of the
  selected tracks and makes a fade on it; here the clip is named, and a
  caller with several clips says so several times. The same gap as the
  velocity's (§229) and the length's (§230), and the same answer.
- **The tree's crossfade is not here.** The tree's `create_fades` does
  two things: it fades a clip inside a range, and it makes a
  **crossfade** where two clips meet inside the range — each clip is
  extended into the other's territory using the **handles** the source
  has beyond its length. A handle is a thing about a file, and this
  tree's clips have no handles: a clip reads the material it names and
  stops. So the fade is made on the clip, and the crossfade is not.
- **The tree's fade bends nothing and this tree's does not either.**
  The tree's `create_fades` writes a fade with a curvature of zero and
  the equal-power shape, and here the same — the bend is the tree's
  `sound.clip.fade`'s business, and this command is the tree's.

### What it cost

- **Four tests** in the sound battery (`tests/fadeto.rs`): the clip is
  faded up to a place, it is faded down to a place, a place outside
  the clip is refused, and an unknown way is refused.
- **One command** (`sound.clip.fade.to`). The sound battery now
  carries **99** commands and the MCP server **310**.

### What it does not settle

- **The tree's crossfade is not here**, and the reason is the handles:
  a clip reads the material it names and stops, so two clips cannot
  lend each other the sound beyond their own ends.
- **The tree's fade is over a selection**, and here it is over a clip:
  the same gap as the velocity's (§229).

## 236. The time of the session

The tree's editor can **insert time** at a place and **cut time** out
of a range. Both are changes to the whole of the session's timeline:
every track moves, every lane moves and every marker moves. Section
228 said this was not here, and named it as the thing that was
missing.

### What was built

- The command `sound.time.insert` puts **silence** into the session at
  a place: the clips at or after it move later by the length of the
  silence, the lanes' breakpoints at or after it move later, and the
  markers at or after it move later — and a marker keeps its span.
- The command `sound.time.cut` cuts a range out of the session: the
  clips inside the range are thrown away, the lanes' breakpoints
  inside it are thrown away, the markers inside it are thrown away,
  and everything after the range moves earlier by the length of it.
- A **locked** clip does not move: a lock says the clip stays where it
  is.
- A range that is **empty** is refused, and so is a silence of no
  length: there is no time to cut and no silence in nothing.

### What the port found

- **The tree's commands are over a selection and this tree's are over
  a place.** The tree's `event.insert_time` and `event.cut_time` take
  a range of the **selection**, and they do it to **every track** of
  the session — the tree's session is a list of tracks and a range of
  it is a fact about the editor. Here the place or the range is
  **given**, and the whole of the document is the thing that moves.
  This is the same gap as the velocity's (§229), the length's (§230)
  and the fade's (§235), and it has the same answer: a place is a
  fact and a selection is the editor's state, which this tree does
  not keep (§200).
- **The tree's clips are in a list and this tree's are nodes.** The
  tree's track holds a playlist of clips and the shift is a field of
  the clip; here a clip is a node under a take and the shift is its
  payload. The **lane** is the same: the tree's lane is a field of
  the track and here it is a node with breakpoints of its own, and the
  tree's `shift_from` moves the breakpoints of a lane where here the
  breakpoints are moved one at a time.
- **The tree's cut removes the clips inside the range and here it does
  too — but the tree's cut does not trim the clips that cross it.**
  The tree's `clear_range` separates the clips at the two edges of
  the range and throws away the ones inside it; the pieces outside
  stay where they were, and the tree's `shift_from` moves the ones
  after the range. Here a clip that crosses the range is **trimmed**
  to the part outside it — the sound of the clip is not lost, and the
  clip that is left says what it plays. That is a difference: the
  tree's piece outside the range is a new clip with a new name, and
  here the clip is trimmed in place.
- **A locked clip does not move here, and the tree's shift says
  otherwise.** The tree's `shift_from` moves "every clip that starts
  at or after `from` by `by`, **locked or not**" — the tree's own
  words. Here a locked clip stays: a lock says the clip stays where
  it is (§ the locks), and a clip that is locked against editing is
  finished. That is a deliberate difference, and the tree's comment is
  the proof that the tree thinks about it.

### What it cost

- **Six tests** in the sound battery (`tests/sessiontime.rs`): the
  silence goes in and everything after it moves, a clip after the place
  moves later, the time is cut out and everything after it moves, a
  clip inside the range is thrown away, a locked clip does not move,
  and an empty range is refused.
- **Two commands** (`sound.time.insert` and `sound.time.cut`). The
  sound battery now carries **101** commands and the MCP server
  **312**.

### What it does not settle

- **The tree's commands are over a selection**, and here they are over
  a place or a range: the same gap as the velocity's (§229).
- **The tree's cut separates the clips that cross the range and here
  it trims them.** A trimmed clip keeps its name and its place; the
  tree's pieces are new clips with new names, and the tree's auto
  rename scheme counts them (§216). Here the cut does not make new
  clips, and so the scheme does not count.
- **The tree's tempo map is not moved by either.** Inserting time
  changes the **samples** the clips are at and not the **ticks** the
  music is at, and the tempo map is in ticks — so the music does not
  change its place in the bar. That is the tree's behaviour and it is
  this one's, and it is worth knowing: inserting silence moves the
  sound and not the music.

## 237. The names of the notes

A MIDI note is a number, and a number is not something a person reads.
The tree's MIDI crate carries a table of names — the note names, the
drum names and the program names — and three earlier sections said
plainly that the names were not here (§217, §219, §223).

### What was built

- The MIDI library gains a module of **names**: `note_name` for a
  pitch, `gm_drum_name` for a drum, `gm_program_name` for a program,
  and `note_or_drum_name` for a note that may be a drum.
- The tables are the **tree's own**, copied and not typed: forty-seven
  drums, from "Acoustic Bass Drum" to "Open Triangle", and a hundred
  and twenty-eight programs, from "Acoustic Grand Piano" to "Gunshot".
  A drum outside the kit has **no** name and a program above the set
  is **"Unknown"** — neither is made up.
- The command `sound.clip.midi.names` reads a clip's notes with the
  names they have, and the programs the clip names, through the
  program-change controllers it holds.
- A clip that holds no notes says so, and it is not a failure: the
  read says `midi: false`.

### What the port found

- **The tree names a note the way a piano names it.** The middle C —
  the pitch of sixty — is **"C3"**, so the octave of a note is the
  octave a piano gives it and not the octave the specification counts.
  The pitch of zero is **"C-2"**, and the tree says so in its own
  words: the Pro Tools convention.
- **The tree's drums are "channel 10" and this tree's drum channel is
  nine.** The tree's `names.rs` says the drum names are for "a pitch
  on channel 10", and its engine counts the channels from zero and
  calls the drum channel nine. Here the channels are counted from zero
  as well (§ the MIDI of a clip), so the drum channel is **nine** in
  both — the tree's comment and the tree's code disagree by one, and
  the code is right.
- **The tree has a library of names and no command that reads them.**
  Its names are called by the panels that show them: the note list of
  the MIDI editor and the sampler window. Here the name is **derived
  where it is read**, so a name cannot go stale against the note it
  names, and a caller that wants the names has a command to ask.
- **The tree's drum name is an option and its program name is a
  string.** A pitch outside the kit has no drum — `None` — and a
  program above the hundred and twenty-eight is the string
  "Unknown". Here the same: a drum outside the kit falls back to the
  note's own name rather than to nothing, because a note on the drum
  channel that is not a drum is still a note.

### What it cost

- **Four tests** in the MIDI library (`tests/names.rs`): the middle C
  is "C3", the pitch of zero is "C-2", the drum of the kit is named and
  the program of the set is named.
- **Three tests** in the sound battery (`tests/midinames.rs`): the
  notes of a clip are named by their pitch, a note on the drum channel
  is named as a drum, and a clip that holds no notes says so.
- **One command** (`sound.clip.midi.names`). The sound battery now
  carries **102** commands and the MCP server **313**.

### What it does not settle

- **The tree's names are shown in panels and here they are read.** A
  panel that lists the notes of a clip would draw the names it reads;
  this tree has no panels for MIDI yet, and the read is what a panel
  would draw from.
- **The tree's octave naming is a convention, not a rule**, and it is
  kept here because it is the tree's: a file written elsewhere may call
  the same pitch "C4", and this tree calls it "C3" because the tree
  does.

## 238. The key of the session

The tree's sound editor has a **key change**: a place in the session
where the key changes, and the key it changes to. The editor's ruler
draws it, and nothing else in the tree reads it — it is a fact for a
person to look at. The tree's MIDI crate reads the key signatures out
of a file, so the number was here and the fact was not.

### What was built

- A **kind** of its own, `sound.key`, in the ruler's row: a key change
  is a thing in the document with a place on the timeline, like the
  tempo and the meter (§ the events of the session).
- The command `sound.key.set` sets the key at a place: the key is
  given by its **name** — "G major", "E flat minor" — and a second
  call at the same place changes the key that is there.
- The command `sound.key.read` reads the keys of the session, and — when
  a place is given — the key **in effect** there: the last change at or
  before it.
- The MIDI library gains the **names of the keys**: a number of sharps
  and a mode are named "C major", and a name is read back into the
  number it stands for.

### What the port found

- **The tree keeps a name and this tree keeps a number.** The tree's
  session holds `key_signatures: Vec<(Samples, String)>` — a place and
  a string — and the string is whatever the caller typed. Here the
  document holds the **sharps and the mode**, because that is what a
  MIDI file carries and what a key signature *is* (§ the MIDI
  library), and the name is **derived** from it: a fact that can be
  worked out is a fact that is worked out, and it cannot go stale
  against the key it names.
- **The tree takes any string and this tree refuses a name that is not
  a key.** The tree's command stores "H major" as happily as "C
  major", and the ruler draws it. Here a name that is not one of the
  fifteen keys of the circle of fifths is refused: a name nobody can
  read is not a key. And a name with **no mode** is refused too — "C"
  on its own does not say whether the music is in C major or in C
  minor, and a name that does not say is not a name of a key.
- **The table of the keys is new work, and it is not the tree's.** The
  tree has no table of keys at all: its key is a free string, and the
  ruler shows whatever it says. The fifteen keys here are the ones a
  MIDI file may carry — seven sharps to seven flats, major and minor —
  and the names of the accidentals are written both ways: "Eb minor"
  and "E flat minor" are the same key.
- **The tree's key is at a sample and this one's is at a tick.** The
  tree's key change is placed on the session's **samples**, like a
  marker; here it is in **ticks**, like the tempo and the meter, because
  a key is a musical fact and the music is counted in ticks (§ the
  events of the session). The difference shows in what happens when
  time moves.
- **Neither tree's key moves when time is inserted, and for different
  reasons.** The tree's `insert_time` moves the clips, the automation
  and the markers, and its `shift_from` says nothing about the key
  changes, so they stay where they are — in samples, in a timeline
  that has just grown under them. Here they stay too, because the
  insertion is in samples and the keys are in ticks: inserting silence
  moves the sound and not the music (§236), and a key is part of the
  music. The same outcome, and a better reason for it.
- **The tree's key is display-only and this one's is read.** The tree's
  ruler draws the key change and no command in the tree reads it back;
  here the key is a fact with a command on it, because a field with no
  command is not a fact.

### What it cost

- **Five tests** in the MIDI library (`tests/keys.rs`): a key is named
  by its sharps and its mode, a signature outside the table has no
  name, a name says which key it is, a name is read the way a person
  writes it, and a name that is not a key is refused.
- **Six tests** in the sound battery (`tests/key.rs`): the key is set
  and read back, it is kept as its sharps and its mode, a second key at
  the same place changes the one that is there, a name that is not a
  key is refused, the key in effect at a place is the last change
  before it, and a session with no key says so.
- **Two commands** (`sound.key.set` and `sound.key.read`) and **one
  kind** (`sound.key`). The sound battery now carries **104** commands
  and **sixteen** kinds, and the MCP server **315**.

### What it does not settle

- **The key does not change the notes.** A key signature says what the
  music is in; it does not transpose anything, and it is not a command
  on a note. The tree's is the same, and the tree's own ruler is the
  only thing that reads it.
- **The table holds fifteen keys and no more.** A key spelled some
  other way — D sharp major written as C sharp major — is a key the
  table does not have, and it is refused rather than guessed at.
- **The key of a MIDI file is still not put into the document.** The
  library reads a file's key signatures and the battery can set one by
  name, but no command yet takes the signatures of an imported file and
  writes them into the session: that is the next thing to close, and
  it is not closed here. (There is now: §239 closed it, and this
  sentence was written before it did.)

## 239. The music of a MIDI file

A MIDI file carries more than notes: it carries a **tempo map** — its
tempos and its meters — and it carries **key signatures**. The tree
reads the first two into the session and throws the third away, and
the export writes the tempo map out and writes no keys at all.

### What was built

- The import grows a parameter, `tempo_map`, and it says whether the
  file's music comes in with it. It does by default, as it does in the
  tree.
- When it does, the file's **tempos** become the document's tempo
  events, its **meters** become meter events and its **key signatures**
  become key events — each at the place the file was imported to plus
  the event's own tick, so that the music of the file lands where the
  notes of the file land.
- The **length** of the imported clip is then worked out under the
  music that came with it: a file at ninety beats a minute makes a
  longer clip than the same file at a hundred and twenty.
- The **export** writes the music out with the notes: the document's
  tempo map, its meters and its keys go into the file, and the notes
  are placed at the ticks they are at in the document rather than at
  the ticks they are at inside the clip.
- The **markers** of the document go into the file as the text markers
  a file carries, at the ticks they are at.
- A file with more than ten thousand tempo, meter and key events
  together is **refused**, as a file with more than ten thousand meter
  changes is refused in the tree.

### What the port found

- **The tree's import leaves the file's tempos behind.** The tree's
  `import_midi_bytes` writes the file's tempos and meters into the
  session's tempo map at the file's **own** ticks — while the notes of
  the same file are moved to the place the file was imported to. So a
  file imported at the tenth minute brings a tempo change that lands
  at the first beat of the session, and the music after the tenth
  minute is played at the wrong speed. Here the events move with the
  notes: an event that stayed behind would say something about a piece
  of music that is not there.
- **The tree drops the key signatures, and so did this tree until
  now.** The tree's import reads the file's key signatures and never
  looks at them; its export writes an empty list of them, so a file
  that goes out of the tree has lost its keys. Here they come in as
  key events (§ the key of the session) and they go out again.
- **The tree's export writes the session's tempo map and this tree
  wrote none at all.** The tree's `export_midi` gathers the session's
  tempos and meters into the file it writes, so that the file plays at
  the speed of the session. Here the export wrote the notes and
  nothing else, and a file of notes with no tempo plays at the
  default — which is not the music.
- **The tree writes the session's markers into the file and this tree
  wrote none.** The tree's `export_midi` gathers the session's markers
  as the file's text markers; here the markers were left behind, and a
  file that goes out of this tree has lost them.
- **The tree's export places the notes at the session's ticks and this
  tree's placed them at the clip's.** The tree adds the clip's own
  tick to every note before it writes the file; here the notes were
  written as they are in the clip, at the clip's own ticks, so a clip
  that was not at the start of the session came out at the start. Now
  the notes are at the ticks they are at in the document.
- **A tempo of a file is a microsecond count, and it comes back very
  nearly what it went in as.** The Standard MIDI File writes a tempo as
  microseconds a quarter note, and ninety beats a minute is six hundred
  and sixty-six thousand and six hundred and sixty-six and two thirds of
  a microsecond — so the tempo that comes back is ninety and a
  fraction. The tree has the same rounding, and it is in the file
  format and not in either tree.

### What it cost

- **Seven tests** in the sound battery (`tests/importmusic.rs`): the
  music of the file comes in with it, the file's key comes in as a
  key, the events land where the notes land, the music may be left
  behind, the exported file carries the music, the exported notes are
  at their place in the document, and the exported file carries the
  markers.
- **No new command.** The import grows a parameter and the export
  grows what it writes; the sound battery still carries **104**
  commands and the MCP server **315**.

### What it does not settle

- **A file still becomes one clip.** The tree makes a **track** for
  every track of the file, each with its own instrument and its own
  clip; here the notes of every track of the file become the notes of
  one clip. The tracks of a file are a separate thing to close. (There
  is now: §240 closed it with a command of its own, and this sentence
  was written before it did.)
- **The tree's import sets an instrument on each track it makes** — a
  drum kit for a track of drums and a subtractive synth for anything
  else — and this tree has no instruments to set.
- **The keys of a file are not a musical truth about the session.**
  They come in because the file carries them, and a session that has
  had two files imported into it has the keys of both, and the later
  one does not replace the earlier one unless they land on the same
  tick.

## 240. The tracks of a MIDI file

A MIDI file holds **tracks**, and they are its parts: a flute and a
drum kit are two tracks of the same piece. The tree's import makes a
track of the session for every track of the file, each with a clip of
its own; this tree's import made one clip with all the notes in it.

### What was built

- A command of its own, `sound.track.midi.import`, which reads a file
  and makes a **track of the session** for every track of the file
  that holds something: a track, a take under it, and a clip in that
  take with the notes of that part of the file.
- The track is named after the track of the file, and a track of the
  file that has no name is named by its number: "MIDI 1".
- A track of the file that holds **nothing** — no notes and no
  controllers — is not a track, and no track is made for it.
- The file's **music** may come in with it, as it does with
  `sound.clip.midi.import`: the tempos, the meters and the keys become
  the document's events at the place the file lands.
- A file that holds **no notes at all** is refused, as it is by the
  clip import: there is nothing to import.

### What the port found

- **The tree's import makes a track for every track of the file and
  this tree's made one clip.** The tree's `import_midi_bytes` walks
  the tracks of the file and, for each of them, adds a track of the
  session with a clip of its own. The notes of a flute and the notes
  of a drum kit are in one clip here, and a clip is one sequence: the
  two parts are one thing, and the tree keeps them apart.
- **The tree names the tracks it makes and sets an instrument on
  them.** The tree's track is named after the track of the file, or
  "MIDI 1" when the file gives it no name, and it carries an
  instrument: a drum kit for a track whose notes are all on the drum
  channel, and a subtractive synth for anything else. Here the name is
  the same and the instrument is not: this tree's processes are named
  by the battery (§ the processes on a clip), and a battery that names
  an instrument it does not have would be a battery that names a
  process it will not do.
- **The tree's import skips a track of the file that holds nothing,
  and so does this one.** The tree's import makes no track for a file
  track that holds neither notes nor controllers, and neither does
  this tree's: a track of nothing is not a track, and a session with
  one in it has a row that can never be heard.
- **The tree's name is bounded at two hundred and fifty-six
  characters.** The tree truncates the name of a track it makes to the
  length the model allows, because a file may name a track with a very
  long string. Here the name is bounded the same way.

### What it cost

- **Four tests** in the sound battery (`tests/importtracks.rs`): every
  track of the file becomes a track, each track holds its own notes,
  the music of the file comes in with it, and a file with no notes is
  refused.
- **One command** (`sound.track.midi.import`). The sound battery now
  carries **105** commands and the MCP server **316**.

### What it does not settle

- **The tracks made here carry no instrument.** The tree's do, and a
  track of notes with no instrument on it makes no sound until one is
  put there.
- **The clip import still makes one clip out of the whole file.** The
  two commands are two ways to read a file: one makes the parts, and
  the other makes a single clip with all of them. The tree has only
  the first, and this tree has both because the clip import was here
  first (§ the MIDI of a clip).

## 241. The mix was not interleaved

A **frame** is a sample of every channel: a stereo sound is two samples
to the frame, and the samples are interleaved. The mix held one sample
of every frame — its buffers were as long as the mix in frames and
were used as interleaved samples — and so a stereo session was **half
as long as it should be**, and every clip was at **half its place**.

This is not an addition; it is a **defect**, and it was found while
building the consolidate (§242).

### What was built

- The buffers of the mix — the track's signal, the buses' signals and
  the master — are as long as the mix in **frames times the channels**.
- A clip's samples go at its place in **frames**: a clip put half a
  second into a stereo session is half a second into the file and not
  a quarter of a second in.
- The **limit** on the length of a mix is checked in frames and not in
  samples, and the mix no longer goes through the audio library's
  mono mixer to be counted.

### What the port found

- **The tree's mix is interleaved, and this one was not.** The tree's
  renderer works in frames and writes the channels out as they come;
  here the buffers were counted in frames and used as samples. A
  **mono** session was right by accident — one channel and one sample
  to the frame are the same number — and a stereo session was wrong:
  half as long, and every clip at half its place.
- **No test of the tree could have caught it, and none of this tree's
  could either.** The tests of this tree that read a rendered file
  read a **mono** one, where the two mistakes are the same mistake.
  The three tests of this arc read a stereo file and count its frames.
- **The fix is in the mix and not in the writer.** The WAV writer
  takes the channels and the samples and works out the frames itself,
  and it was right: it wrote a file of half the frames because half
  the samples were given to it.

### What it cost

- **Three tests** in the sound battery (`tests/mix.rs`): a stereo clip
  is where it was put, a mono clip is where it was put, and two stereo
  clips are heard together.

### What it does not settle

- **The mix is still one mix.** The tree's mixer can route a channel
  to another channel — a **matrix** — and this one cannot; the tree's
  session format and its source-to-track-to-session routing are a
  redesign rather than an addition, and it is not done here.

## 242. The consolidate of a track

The tree's consolidate renders the sound of a track's clips over a
range, writes it to a file, and puts one clip of that file where the
clips were. The sound is the same and the editing is not.

### What was built

- The command `sound.track.consolidate` renders the sound of a
  track's clips over a range, writes it to a file in a directory, adds
  the file to the document as a **source**, and puts **one clip** of
  that source where the clips were.
- The sound that is rendered is the sound of the track's **clips**:
  the track's trim and the track's inserts are not in it, because they
  belong to the track and the track is still there.
- A clip that **crosses the edge** of the range keeps the part outside
  it: the head of the first clip and the tail of the last stay where
  they were.
- The **MIDI** clips of the track are left alone: they hold no sound
  to render, and a MIDI clip is not a file.
- A range with no length is refused, and so is a node that is not a
  track.

### What the port found

- **The tree's consolidate renders the clips and this one does too.**
  The tree's `consolidate` renders the track's clips over the range
  with `render_clips` — the clips' own gains and fades, and not the
  track's strip — and puts the result in a new clip on the same
  track. Here the mix is asked for the track's **raw** signal: the sum
  of its clips, before the trim and before the inserts. The strip
  still applies to the file that comes out, exactly once.
- **The tree's consolidate separates the clips that cross the range
  and this one trims them.** The tree's `clear_range` cuts a clip that
  crosses into two pieces and keeps the outside one; here the clip
  keeps its head or its tail and stays one clip (§ the cut of a MIDI
  clip, and § the time of the session: the tree separates and this
  tree trims).
- **The tree's consolidate is over a selection and this one is over a
  range.** The same gap as the velocity's (§229), the length's (§230)
  and the fade's (§235), and the same answer: a range is a fact and a
  selection is the editor's state.
- **The tree's sources are in a pool and this tree's are files.** The
  tree's consolidated sound is an `AudioBuffer` in the session's
  source pool; here it is a file on the disk, a `sound.source` node
  and an asset beside the document — because that is how every source
  of this tree is a file (§ the source of a clip).

### What it cost

- **Five tests** in the sound battery (`tests/consolidate.rs`): the
  clips of the track become one clip, the file holds the sound of the
  range, a clip that crosses the range keeps the part outside it, an
  empty range is refused, and a node that is not a track is refused.
- **One command** (`sound.track.consolidate`) and one thing in the
  mix: the **raw** signal of a track, the sum of its clips before the
  strip. The sound battery now carries **106** commands and the MCP
  server **317**.

### What it does not settle

- **The tree's commit is not here.** The tree's `track.commit` renders
  a track through its inserts into a **new track** and marks the old
  one inactive and hidden; this tree has no inactive track and no
  hidden track, and the consolidate keeps the track it was given.
- **The sound of the range is written to a file and not kept in the
  document.** A document that is copied to another machine carries the
  file beside it, as every source of this tree does.

## 243. The taking away of a node

The tree's editor can delete a clip, a marker and a track. This tree
could do **none** of it: a thing that could be made but not unmade is a
thing that is there for ever, and a document that cannot forget is a
document that grows.

### What was built

- The command `sound.clip.remove` takes a **clip** away. Its edges go
  with it: a clip that played a source is a clip that plays nothing,
  and the edge that said so is an edge to nothing.
- The command `sound.marker.remove` takes a **marker** away. Nothing
  else goes with it: a marker is a place and nothing points at it.
- The command `sound.track.remove` takes a **track** away, with its
  **takes**, its **clips** and its **lanes**, and the **routes** and
  **sends** that named it.
- A clip whose **content** is locked is **refused**, and so is a
  **master**: the master holds the fader of the whole mix.

### What the port found

- **The tree deletes a clip over a selection and this tree names it.**
  The tree's `edit.clear` takes the selected clips away — or the
  clips in a range — and the tree's `track.delete` takes the selected
  tracks away, repairing the routes that pointed at a deleted bus.
  Here the thing is **named**: a clip, a marker and a track are three
  different things and each of them has a command of its own. The same
  gap as the velocity's (§229) and the length's (§230), and the same
  answer.
- **The tree's markers are deleted by their number and this tree's by
  their id.** The tree's marker is a memory location with a number in
  the ruler; here a marker is a node and the node's id is its name.
- **The tree's `edit.clear` takes a locked clip away and this tree
  refuses.** The tree's clear does not ask about the locks: it removes
  the clips in the range, locked or not. Here a clip whose content is
  locked is refused, because a lock says the clip's sound is finished
  (§ the locks) and a finished clip is not thrown away. That is a
  deliberate difference.
- **The tree repairs the routes that named a deleted track and this
  tree lets them go.** The tree's `track.delete` reroutes the tracks
  that fed a deleted bus to the bus that is left. Here the routes and
  the sends that named the track go with it: a route to a track that is
  not there is a route to nothing, and a document that keeps it is a
  document that says something that is not true.

### What it cost

- **Six tests** in the sound battery (`tests/remove.rs`): a clip is
  taken away, a locked clip is refused, a marker is taken away, a
  track is taken away with its clips and its route, a master is
  refused, and a node that is not a clip is refused.
- **Three commands** (`sound.clip.remove`, `sound.marker.remove` and
  `sound.track.remove`). The sound battery now carries **109** commands
  and the MCP server **320**.

### What it does not settle

- **A take cannot be taken away on its own.** The tree's
  `track.playlist_delete_unused` throws away the takes of a track that
  are not used; here a take goes when its track goes, and not before.
- **The groups of a track keep their name when the track goes.**
  The tree's `track.delete` takes the deleted tracks out of their
  groups; here the group node stays and the edge to the track goes
  with the track, which is the same thing said the other way round.

## 244. The onsets in the spectrum

Section 221 said that a transient is found by the **level** here and by
the **spectrum** in the tree, and it said why: the tree's detector
needs a fast Fourier transform, and this kit's crates carry no
dependencies at all.

The transform is a piece of work, and it is now done — **in the kit's
own crates, with no dependency at all**.

### What was built

- The audio crate gains a **transform**: a fast Fourier transform of a
  frame, written here — the samples are put in a bit-reversed order
  and folded together in stages, and the work is `n log n`. A length
  that is not a **power of two** is refused, because there is no answer
  and not a wrong one.
- The audio crate gains the tree's **onset detector**: the sound is
  read in frames of about twelve milliseconds, each frame is
  transformed, the power at each frequency is gathered into **bands** of
  twenty-four to the octave, each band's level is a logarithm, and the
  **flux** of a frame is how much the bands rose against the loudest
  of their neighbours.
- A frame is an onset when its flux stands out from the flux around it
  by a **fixed amount** — and not by an amount relative to the loudest
  hit, so that a quiet hit still counts — and the onset is then placed
  at the **attack**: the place where the energy first rises.
- The sound battery's separation at the transients uses it now.

### What the port found

- **The tree's detector is now this tree's detector, and the tree's
  own tests are this tree's own tests.** Six of the tree's tests came
  over: notes that start over a ringing note are placed on their
  attacks, the quieter hits between loud ones count, a held chord with
  vibrato and chorus has no transients, the transients do not depend on
  where the analysed audio starts, a fast release is not a transient,
  and channels that cancel when mixed still show their transients. They
  all pass against the transform written here.
- **The tree's threshold is fixed and this tree's was relative.** The
  tree's detector asks a peak to stand out from the flux around it by
  a **fixed** amount, so a quiet hit counts even beside a loud one.
  This tree's level detector asked a frame to reach a fraction of the
  **whole sound's** level, and a hit that was quiet beside a loud one
  was not heard at all. That is the difference the tree's rule is
  about, and it is now the tree's rule.
- **The tree's sensitivity is the same knob and it means the same
  thing.** It is the size of the fixed amount: ten at the bluntest,
  two and a half in the middle, and five sixteenths at the most
  sensitive. A hit sixty decibels below another is not heard by the
  blunt detector and is heard by the sensitive one — and that is
  measured, not assumed.
- **An abrupt stop is an onset too.** A square wave that stops dead
  has an edge at the stop, and the edge is broadband: the bands rise
  at it, and the detector calls it an onset. The tree's detector does
  the same, and the tree's own guard is against a **fast release** —
  a gradual one, whose attack edge is weak and whose energy does not
  grow — and not against a stop that is a click.

### What it cost

- **Four tests** in the audio crate (`tests/fft.rs`): a frame of sound
  has power, a frame of silence has none, a sine has its power at its
  own frequency, and a length that is not a power of two is refused.
- **Six tests** in the audio crate (`tests/onsets.rs`): the tree's
  own six, brought over.
- **Three tests** in the sound battery (`tests/transients.rs`): the
  clip is separated at its transients, a clip that plays no sound is
  refused, and the sensitivity is how small a rise may be.
- The sound battery carries **109** commands and the MCP server **320**.

### What it does not settle

- **The level detector is still in the library.** It is the detector
  the sound battery used, and it is a **read** of a sound's shape: it
  is what says which parts of a sound are loud (§ the stripping of
  silence). It is not the detector of the transients any more.
- **The clip's own sound is still not what is analysed.** The
  material is read and not the clip's rendered sound, so a clip that
  is pitched or processed is cut at the attacks of the **material**
  (§221).

## 245. The split into mono

A stereo track is two channels that travel together, and a track of one
channel is two things that can be heard apart. The tree's editor splits
a stereo track into a track for each of its channels.

### What was built

- The command `sound.track.split.mono` makes a **track** for each
  channel of a track: each of them is **mono**, each of them is panned
  to its side — the left one hard left and the right one hard right —
  and each of them plays the channel of the material that is its own.
- The channel is written to a **file** of its own, beside the session:
  this tree's sources are files and not a pool of sounds, so the
  channel of a source is a file.
- The **original** track is left where it is: it is the caller's to
  mute, and the tree leaves it too.
- A track that is **not** stereo is **refused**: there is nothing to
  split, and a command that quietly does nothing is a command that
  answers a question nobody asked.
- The **mix** learned the tree's rule for a narrow source: the mix is
  as wide as the **widest** source, and a source that is narrower is
  heard in every channel of the mix — the tree's renderer takes the
  source's channel `c % n` for the mix's channel `c`.

### What the port found

- **The tree's split writes a file for each channel and so does this
  one — but the tree's files live in a pool.** The tree's split copies
  the channel of the source into a new source of the session's pool,
  and the copies are shared between the clips that need them. Here a
  source is a **file** on the disk (§ the source of a clip), so the
  channel is a file: "<source>.L.wav" beside the session, with the
  asset that carries it.
- **The tree's split keeps the whole channel.** Its own words: the
  cloned clips keep their offsets, their stretch, their gain and their
  fades, and every reader sees genuine mono media. Here the same: the
  mono file holds the whole channel, and the clips that play it keep
  everything they had.
- **The tree's mix lets a narrow source be narrow and this one
  refused it.** The tree's renderer takes the source's channel `c % n`
  for the track's channel `c`, so a mono source in a stereo track is
  heard in both channels. Here the mix **refused** a document whose
  sources did not all have the same number of channels — a mono source
  in a stereo session was not a document that could be played at all.
  The mix is now as wide as the widest source, and a narrower source
  is heard in every channel of it.
- **The tree's split is over a selection and this one names a track.**
  The same gap as the velocity's (§229) and the taking away's (§243),
  and the same answer.

### What it cost

- **Five tests** in the sound battery (`tests/split.rs`): a stereo
  track is split into two, the two are panned to the sides, each track
  plays one channel of the material, the original track is left where
  it is, and a mono track is refused.
- **One test** in the sound battery (`tests/mix.rs`): a mono source is
  heard in both channels of a stereo mix.
- **One command** (`sound.track.split.mono`). The sound battery now
  carries **110** commands and the MCP server **321**.

### What it does not settle

- **The new tracks carry none of the old one's settings.** The tree's
  new track is a fresh one with a pan and nothing else, and this one
  is the same: the fader, the mute and the sends are the caller's to
  set again.
- **The tree's split copies the clips into the new tracks and keeps
  the original's auto-rename scheme; this one's copies do not.** The
  scheme is a node and the copies are not in it (§ the auto rename of
  a clip), and nothing has been said about them yet.

## 246. The repeating of a range

The tree's editor repeats a selection: it copies the material of the
range and lays the copies after it, as many times as the caller asks
for.

### What was built

- The command `sound.track.repeat` copies the clips of a track that
  overlap a range and lays the copies **after** the range, as many
  times as the caller asks for.
- A copy is **trimmed to the range**: it is as long as the range and it
  plays the material from where the range began.
- A copy that is trimmed at its head loses its **fade in** — the fade
  was of the material before it — and a copy that is trimmed at its
  tail loses its **fade out**. A copy keeps everything else: its gain,
  its stretch, its pitch, its processes and its **beat**, which follows
  the audio as it does everywhere else (§ the beat of a clip).
- The copies keep the **source** of the clip they were made from: a
  copy that plays nothing is a copy of nothing.
- The **count** is clamped into its bounds — one to nine hundred and
  ninety-nine, as the tree's is — because it is a knob and not a fact
  about the material.

### What the port found

- **The tree's repeat is over a selection and this one is over a
  range.** The same gap as the velocity's (§229), the taking away's
  (§243) and the split's (§245), and the same answer: a range is a
  fact and a selection is the editor's state.
- **The tree's paste clears what it covers and this one's copies do
  not.** The tree's `paste` places a clip with `place_clip`, and
  `place_clip` clears the range the clip covers first: a repeat that
  lands on material throws that material away. Here the copies are laid
  **over** what is there, as the loop's copies are (§ the loop of a
  clip): a document that throws away material nobody asked it to throw
  away is a document that surprises.
- **The tree's copy of a range drops the fades at the edges and so
  does this one.** The tree's `copy_range` trims the clips to the
  range and clears the fade at each end it cuts: a fade in is of the
  material before the clip and a fade out is of the material after it,
  and a copy that has neither is not missing anything.
- **The tree's repeat counts in the tree's own shuffle mode.** The
  tree's repeat moves the material to make room when the editor is in
  shuffle mode; this tree has no shuffle mode, and the copies are laid
  where they are.

### What it cost

- **Five tests** in the sound battery (`tests/repeat.rs`): the
  material of a range is laid again, the range is repeated as many
  times as asked, a copy is trimmed to the range, a copy plays the
  source of the clip it was made from, and an empty range is refused.
- **One command** (`sound.track.repeat`). The sound battery now
  carries **111** commands and the MCP server **322**.

### What it does not settle

- **The copies are not in the auto rename scheme.** A copy is a new
  clip and the scheme is a node with edges to its clips (§ the auto
  rename of a clip); the copies of a repeat are not in it, and their
  names are the names of the clips they were made from.
- **The tree's repeat copies the material of every selected track and
  this one repeats one track.** A caller that wants two tracks
  repeated says so twice.

## 247. The clipboard

The tree's editor has a clipboard: it copies the material of a range,
and pastes it somewhere else. This tree's battery had **no clipboard at
all** — there was no copy and no paste — and the copy and the paste are
here now.

### What was built

- The command `sound.track.copy` copies the material of a range of a
  track into the clipboard, `sound.track.cut` copies it and then takes
  it away, `sound.track.paste` lays the clipboard onto a track, and
  `sound.track.clipboard.read` says what the clipboard holds.
- The clipboard lives in the **document's payload**, under a key of its
  own: `sound.clipboard`. The document is the only place this tree
  keeps a thing, and a key of its own means that two batteries cannot
  collide by accident.
- It is **not a node**: a clipboard is not a thing in the session, it
  is a thing that was **copied out of** it. What it holds is the
  **payloads** of the clips of the range — their own words and the
  source they play — the **automation** of the range and the **length**
  of the range.
- A copy is **trimmed to the range**, and a copy trimmed at its head
  loses its **fade in** — the fade was of the material before it — and
  a copy trimmed at its tail loses its **fade out**. The **beat**
  follows the audio, as it does everywhere else.
- The paste makes **new** clips, with the source they play, and lays
  them **over** what is there.
- A range with **no length** is refused: there is nothing in it. An
  **empty** clipboard is refused: there is nothing to paste. A range
  with no material in it copies nothing and says so.
- The **cut** takes the material away: the clips inside the range are
  thrown away, and the clips that cross its edge keep the part outside
  it.

### What the port found

- **The tree's clipboard is the engine's state and this one is the
  document's.** The tree's `edit.copy` is a *noundo* command: it does
  not change the document, and so it cannot be undone. Here the
  clipboard is in the document, which means the copy **changes** the
  document and is undoable like any other change — and it also means
  that a document that is **saved** carries what was copied in it. The
  tree's clipboard dies with the session; this one travels with the
  file. Neither is obviously right, and the difference is the price of
  putting the clipboard where this tree keeps everything else.
- **The tree's clipboard holds much more.** It carries the
  **automation** of the range, the **markers** in it, the **clip gain**,
  the **clip effects**, the **grid offset** and the **auto rename
  schemes** of the copied clips. Here it carries the clips, the
  **automation** and the length, and the section says why: this tree
  keeps a track's automation in **lanes** (§ the editing of a lane) and
  its markers as **nodes** in the document, so a clipboard that
  carried the markers would carry a second truth about the same
  timeline — but the lanes' breakpoints are the automation itself, and
  a copy of them is a copy of the curve (§251). The grid offset is the
  editor's state, and the same gap as the repeat's (§246): a range is
  a fact and a selection is not.
- **The tree's copy is over several tracks and this one is over one.**
  The tree's clipboard holds a *list of tracks*, and its paste lays
  them on several tracks, cycling the clipboard's tracks. Here a
  clipboard is one track's range, and a paste is onto one track.
- **The tree's paste, in shuffle mode with a selection, clears the
  selection first.** The paste replaces what it covers. Here a paste
  lays the material **over** what is there, the same difference the
  repeat has (§246): a document that throws away material nobody
  asked it to throw away is a document that surprises.
- **The tree's scheme carries an id** "to tell this scheme from
  another one under the same key after an undo or in another session,
  for pastes from the clipboard". The clipboard is here now, and the
  schemes it holds are told apart by their **nodes**: a node is the
  thing, and the tree's id is a key under a key.

### What it cost

- **Six tests** in the sound battery (`tests/clipboard.rs`): the range
  is copied, the copy changes nothing, the clipboard is pasted onto a
  track, the cut copies and removes, an empty clipboard is refused and
  an empty range is refused.
- **One module** (`ranges.rs`) that the **repeat** now shares with the
  **copy** and the **cut**: a range means one thing and not three, and
  the trimming of a copy is written once.
- **Four commands** (`sound.track.copy`, `sound.track.cut`,
  `sound.track.paste`, `sound.track.clipboard.read`). The sound
  battery now carries **115** commands and the MCP server **326**.

### What it does not settle

- **The markers of the copied range do not travel.** A paste brings
  the clips and the automation and nothing else: no marker, no clip
  gain, no clip effects.
- **The auto rename schemes of the copied clips do not travel.** The
  pasted clips are new clips and they are **not** in the scheme (§ the
  auto rename of a clip); their names are the names of the clips they
  were made from, and the tree's scheme id — the thing that would
  have told two schemes apart under the same key — is not here, because
  a node is the thing.
- **The paste does not clear what it covers**, and the tree's does.
- **One track at a time.** A caller that wants two tracks copied cuts
  and pastes twice, and the two pastes are two commands.

## 248. The voice of a track

The tree's track header says **"dyn"** or **"off"**, and "off" makes
the track **inactive**. An inactive track is not heard, and it is not
gone: it is in the document with its clips and its settings, and a
command can make it play again.

### What was built

- The command `sound.track.inactive` turns a track's **voice** off and
  on again. The voice is a **field of the track** — the tree's track
  carries it as a field too — and it is read back from the track's own
  data.
- An inactive track is **not heard**: it is not in the mix, not on a
  bus, not a **pre-fader** send's source and not a **stem**. A reader
  that reads a bus is a track, and its voice is off too.
- The track itself is **not touched**: its clips, its fader, its pan
  and its automation are where they were, and turning the voice on
  again brings the track back exactly as it was.
- A node that is **not a track** is refused.

### What the port found

- **The tree's commit makes the old track inactive *and* hidden.** The
  tree's `commit_track` writes the track's sound to a file, makes a
  new track of it, and sets the old track's `inactive` and `hidden`.
  The **hidden** is not here: this tree draws no track headers, and a
  thing that nothing draws and nothing reads is not a fact. The
  **inactive** is a fact — it is what makes the old track stop being
  heard — and so it is here.
- **The tree's inactive track is skipped in the mix, and so is this
  one.** The tree's mixer filters `!t.inactive` over the tracks, over
  the aux tracks and over the masters, and the solo counts only the
  tracks that are active. Here the filter is in the same places: the
  audio tracks, the readers and the masters.
- **The tree's commit hides the old track because it is *looking* at
  the new one.** A commit is a thing a person does in front of a
  window, and the window shows the new track under the old one. Here
  the commit is a command and not a window, and a command that leaves
  two tracks in the document is the honest thing: the old track is
  there, and it is quiet.

### What it cost

- **Six tests** in the sound battery (`tests/inactive.rs`): the track
  carries its voice, the voice is turned off and on again, an inactive
  track is not heard, an inactive track is not gone, an inactive track
  is not bounced, and a node that is not a track is refused.
- **One field** on the track's payload (`inactive`) and **one
  command** (`sound.track.inactive`). The sound battery now carries
  **116** commands and the MCP server **327**.

### What it does not settle

- **A track that is inactive is not in the mix, but it is still in the
  document** — and a document that is rendered, bounced or saved says
  nothing about it. The tree's commit hides it, and hiding is not
  here (§ the commit of a track).
- **The voice is not a mute and it is not a solo.** A muted track is
  silent and a solo silences the others; an inactive track is **not
  there** — it is not a source for a send either, so a bus fed by an
  inactive track hears nothing.

## 249. The commit of a track

The tree's **commit** writes the sound of a track to a file, makes a
**new track** of that file, and turns the **voice** of the old track
off. The editing is over: the old track is still there, with its clips
and its settings, and it is quiet.

### What was built

- The command `sound.track.commit` writes the sound of a track to a
  file in a directory and makes a new track of that file.
- The sound that is written is the sound of the track **before its
  fader**: the clips, through the track's **trim** and through the
  track's **inserts**, and not through its **mute**, not through its
  **fader** and not through its **pan**. The tree's commit renders a
  track before its fader, and this one does the same.
- The new track carries the old track's **fader** and its **pan** and
  its **sends**, and it **feeds the same bus** when the old track fed
  one. The sound of the document is therefore the sound it was: the
  level is not counted twice.
- The old track is **not taken away**: its voice is turned off, and it
  is in the document with its clips and its settings.
- The new track takes the **place** of the old one — the document's
  order is the order the mixer works in, and a committed track stands
  where the track it was committed from stood.
- A track with **no sound** is refused, and so is a track whose voice
  is already off: there is nothing to write down.

### What the port found

- **The tree's commit renders two seconds more than the material.**
  The tree's `commit_track` renders the track over a range that ends
  two seconds after its last clip, so that the tail of an insert is in
  the file. Here the mix is as long as the material — the tree's
  renderer renders a range and this one's mix is the length of the
  document — so the tail past the last clip is not in the committed
  file.
- **The tree's commit hides the old track as well as making it
  inactive.** The SDK has a `hidden` flag for a node that is not
  drawn, and the flag is about drawing: the sound battery draws
  nothing, so a hidden track would say nothing about the sound. The
  **inactive** is a fact about the sound, and it is here (§248).
- **The tree's commit does not copy the pan, and this one does.** The
  tree's pre-fader render centres the pan and the new track keeps the
  default, so the sound is the same. Here the pan is applied in the
  mix and not in the render, so the committed file has no pan in it —
  and the new track keeps the old track's pan, and the sound is the
  same for the same reason.
- **The tree's commit copies the mixer's volume, its output and its
  sends, and not its inserts.** Here the new track carries the
  **fader**, the **pan** and the **sends**, and the route to the bus,
  and it has no inserts: the inserts are in the file, and the old
  track keeps its own.
- **The tree's commit says nothing when it does nothing.** The tree's
  `commit_track` gives back `None` for a track that is not an audio
  track and for a track with no material, and the command answers
  with an empty list and no word. Here it is **refused**, and it says
  why: a command that does nothing and says nothing is a command that
  cannot be told from a command that was never run.

### What it cost

- **Seven tests** in the sound battery (`tests/commit.rs`): the commit
  makes a new track of a file, the old track is quiet and it is still
  there, the sound is the same after the commit, the fader is not
  counted twice, the committed track takes the place of the old one, a
  track with no sound is refused, and a node that is not a track is
  refused.
- **One command** (`sound.track.commit`). The sound battery now
  carries **117** commands and the MCP server **328**.
- The **mix** learned to give back the signal of a track **before its
  fader** when a caller asks for it: the tree's commit needs that
  sound, and the mix is the only place it is made.

### What it does not settle

- **The tail of the material is not in the file.** The committed sound
  is as long as the track's clips, and the two seconds of tail that
  the tree's commit renders are not here.
- **The automation of the inserts is not in the file.** The tree's
  pre-fader render keeps the track's **plugin automation** and runs
  the inserts under it; here the inserts run with the parameters they
  have, and the track's own automation of the fader, the mute and the
  pan is not in the file either — the fader and the pan are on the new
  track and the mute is not.
- **The mute of the old track is not in the file and not on the new
  one.** The tree's commit renders a muted track as if it were not
  muted, and here the committed sound is the track before its mute as
  well; the new track is not muted, and the old track keeps its mute.
- **The file is a WAV** — v2's sources are files on the disk and a
  sound is a sound — and the new track is named `<name>.cm`, which is
  the tree's name for a committed track.

## 250. The chain of inserts

The tree's **insert** is a plugin in a **slot** on a track: there are
so many slots, some of them are empty, and an insert is taken away by
emptying its slot. Here the insert is a **node** under the thing it is
on, and the **chain** is the order of those nodes.

### What was built

- The command `sound.insert.remove` takes an insert away: the node is
  gone and the sound is the sound without it. There is no slot left
  empty, because there was never a slot.
- The command `sound.insert.move` moves an insert: it and another one
  **swap their places** in the chain, and the chain is the order the
  plugins are run in.
- Two inserts of **different** things are refused: a chain is the
  chain of one thing, and there is no chain that holds both.
- A node that is **not an insert** is refused.

### What the port found

- **The tree's inserts live in slots and this tree's are nodes.** The
  tree's track carries a list of slots — a fixed number of them, with
  some empty — and an insert is at an index. Here an insert is a node
  under the track, the chain is the order of the children, and a
  taken-away insert leaves nothing behind. The tree's empty slot is a
  hole in a fixed ladder; here the chain is only as long as it is.
- **The tree's move swaps two slots and this one swaps two nodes.**
  The tree's `insert_move` swaps the slots `from` and `to` of one
  track, and says nothing when a slot is empty. Here the two inserts
  swap their places in the chain of the thing they are on, and the
  inserts have to be on the same thing: the tree's slots are the
  track's by construction, and here a node can be on a track or on a
  clip.
- **The tree's removal of an insert that is not there does nothing**,
  and answers with an empty object. Here a node that is not an insert
  is **refused**, and it says why.

### What it cost

- **Four tests** in the sound battery (`tests/inserts.rs`): the insert
  is taken away, two inserts swap their places, a node that is not an
  insert is refused, and two inserts of different things are refused.
- **Two commands** (`sound.insert.remove` and `sound.insert.move`).
  The sound battery now carries **119** commands and the MCP server
  **330**.

### What it does not settle

- **The chain is not a ladder of fixed slots.** A track here may have
  as many inserts as it likes, in any order, and there is no slot
  number to name: an insert is named by its **node**.
- **The tree's insert has a preset and this one's has not.** The
  tree's `mix.insert_param` and its preset command put a named preset
  on an insert; here an insert is a plugin and a table of numbers.

## 251. The automation of a range

The tree's clipboard carries the **automation** of the range it copied:
the breakpoints of the track's lanes that were inside the range, and a
paste puts them on the track it pastes onto. The tree's cut takes them
away with the material. Here the clipboard carried the clips and the
length and nothing else, and that was wrong.

### What was built

- The **copy** of a range carries the **breakpoints** of the track's
  lanes that were inside it, moved to the range's own head.
- The **paste** writes them on the track: the breakpoints are put
  where the material was put, and the breakpoints that were **there**
  are taken away — the tree's paste clears the range it covers before
  it writes, so that a paste is a paste and not the sum of two curves.
- The **cut** takes the automation of the range away with the
  material.
- A range with a **curve** in it and no clip in it has something to
  copy: the material of a range is its clips **and** its automation.
- The **read** of the clipboard says how many lanes it holds and what
  is in them.

### What the port found

- **The tree's copy carries the automation and this one's did not.**
  The tree's `edit.copy` gathers the breakpoints of every lane of the
  track that fall inside the range and puts them in the clipboard with
  the clips. The port of the clipboard (§247) left them out, and the
  reason it gave — "a clipboard that carried them would carry a second
  truth about the same timeline" — was **wrong**: the lanes *are* the
  automation, and a copy of a lane's breakpoints is a copy of the
  curve and not a second curve. The truth is that the tree's clipboard
  carries the **markers** as well, and those are nodes in the
  document, and a copy of those would be a second truth.
- **The tree's cut takes the automation away when the session says
  that the automation follows the edit.** The tree's session carries a
  flag for it, and the flag is the editor's state. This tree has no
  such flag, and a cut takes the curve of the range away: the material
  and the automation are one thing, and one of them without the other
  is a document that says two things about the same time.

### What it cost

- **Three tests** in the sound battery (`tests/clipboard.rs`): the
  automation of a range is copied, the paste writes the automation, and
  a cut takes the automation away.
- **No new command**: the copy, the cut and the paste are the same
  three commands, and they carry one thing more. The sound battery
  carries **119** commands and the MCP server **330**.

### What it does not settle

- **The tree's copy carries the markers, the clip gain and the clip
  effects**, and this one's does not: a paste brings the clips and the
  curve and nothing else.
- **The tree's repeat copies the clips and not the automation, and so
  does this one's repeat** (§246): the repeat lays the material again
  and the curve stays where it is.

## 252. The name of a track and of a clip

The tree's editor renames a track and renames a clip. The name is what
a person calls the thing, and this tree's battery had no way to change
one — the name was fixed when the thing was made.

### What was built

- The command `sound.track.rename` renames a track, and
  `sound.clip.rename` renames a clip. The name is trimmed, and the
  name is put on the node.
- A name that is **empty** is refused: a name of spaces is no name.
- A name that is **longer than the tree's bound** — two hundred and
  fifty-six characters — is refused.
- A node that is **not a track** is refused, and so is a node that is
  not a clip.
- A clip that is in an **auto rename scheme** keeps its place in it: the
  scheme names the pieces of a cut, and a clip that is renamed by hand
  is still a clip of the scheme.

### What the port found

- **The tree's names are unique and this tree's are not.** The tree's
  `track.rename` refuses a name that another track already has, because
  the tree's tracks are named and a caller names them. Here the **id**
  is the identity and the name is a label: two tracks may carry the
  same name, and the tree's rule is not here because a document that
  says the same thing twice about a name is not a document that says
  two things about the same thing.
- **The tree's rename throws the end of a long name away, and this one
  refuses it.** The tree takes the first two hundred and fifty-six
  characters of a name — sixty-four for a track, in the tree's own
  command — and the rest is lost. A name that was cut in half is not
  the name that was asked for, and here the name is **refused**.
- **The tree's rename of a clip does not take it out of its auto rename
  scheme, and this one's does not either.** The tree's rename sets the
  name and leaves the scheme alone; the scheme names the pieces of a
  later cut, and the clip that was renamed by hand is still a clip of
  the scheme.

### What it cost

- **Six tests** in the sound battery (`tests/rename.rs`): the track is
  renamed, the clip is renamed, an empty name is refused, a long name
  is refused, two tracks may have the same name, and a node that is
  not a track is refused.
- **Two commands** (`sound.track.rename` and `sound.clip.rename`). The
  sound battery now carries **121** commands and the MCP server
  **332**.

### What it does not settle

- **The names are not unique.** The tree refuses a name that is taken;
  here two tracks may have the same name, and a caller that finds a
  track by its name finds the first one with that name.
- **The tree's rename of a track renames its bus as well**, when the
  track is a folder that feeds one. There are no folders here.

## 253. The settings of a send

The tree's send has a command for its **level** and another for the
rest of it. This tree's battery could make a send and could take it
away, and the level could only be set at the moment the send was made:
a send that wanted to be quieter afterwards had no way to say so.

### What was built

- The command `sound.send.set` changes a send: its **level**, its
  **pan**, whether it is **silent**, and whether it is taken **before
  the fader**.
- A level outside the tree's range — minus one hundred and forty-four
  decibels to twelve — is **refused** and not pulled to the edge, and
  a pan outside minus one and one is refused too.
- A node that is **not a send** is refused.

### What the port found

- **The tree's send has a command for its level and this one had
  none.** The tree's `mix.send_level` sets the level of a send, and
  the tree's `mix.send` makes one. Here the level could only be given
  when the send was made, so a send's level was a fact that could
  never change — and a fact that can never change is not a fact about
  a send, it is a fact about the moment the send was made.
- **The tree clamps the level and this one refuses it.** The tree's
  `mix.send` pulls a level outside its range to the edge; here it is
  refused, the same rule the fader follows (§ the fader of a track).
- **The tree's send is a slot and this one's is a node.** The tree's
  track carries a list of send slots and a send is at an index; here
  the send is a node under the track and the bus it goes to is a
  relation. The tree's level command names the **slot**, and this one
  names the **node**.

### What it cost

- **Five tests** in the sound battery (`tests/sendlevel.rs`): the level
  of a send is changed, a level outside the bound is refused, a pan
  outside the bound is refused, a send may be silent and pre-fader, and
  a node that is not a send is refused.
- **One command** (`sound.send.set`). The sound battery now carries
  **122** commands and the MCP server **333**.

### What it does not settle

- **The tree's send has a preset and a target that may be changed**,
  and this one's has neither: the bus a send goes to is fixed when the
  send is made, and a send that should go somewhere else is made
  again.
- **The tree's `mix.mute_sends` mutes every send of a track at once**,
  and this one's does not: a caller mutes the sends one by one.

## 254. The duplicate of a track

The tree's editor duplicates a track: the copy has the same clips, the
same automation, the same inserts and the same sends, and it stands
where the old one stands. This tree's battery could not.

### What was built

- The command `sound.track.duplicate` copies a track. The copy is a
  **new track** — a new set of clips with their own ids — and it
  carries the old one's **clips**, its **automation**, its **inserts**
  and its **sends**. Each of them may be left out.
- The **sources** are shared: a sound is a file, and a file may be
  played twice.
- The copy **feeds the same bus** when the old track fed one, and it
  stands **where the old one stands** — the document's order is the
  order the mixer works in.
- A **master** is refused: it holds the fader of the mix and not a
  sound of its own, and two of them would be two faders over one mix.
- A node that is **not a track** is refused.

### What the port found

- **The tree's copy is a copy of the track's own struct, and this one's
  is a copy of what the track holds.** The tree's track is a value in
  a list, and the duplicate clones it and gives the clips new ids. In
  this tree a track is a **node** with children — takes, clips, lanes,
  inserts, sends — and the relations that go with them: a copy of the
  node is a copy of the children, and the relation from a clip to its
  source is made again for the new clip.
- **The tree's copy keeps the tree's own name and makes it unique**:
  `drums` becomes `drums.dup`, and if that is taken it becomes
  `drums.dup 2`. Here the names are labels and not keys (§252), so
  the copy is named `drums.dup` and two of them may have that name.
- **The tree's copy carries the track's output route**, because the
  route is a field of the tree's mixer. Here the route is a
  **relation**, and the copy's relation to the bus is made again.

### What it cost

- **Six tests** in the sound battery (`tests/duplicate.rs`): the copy
  carries the track, the copy is a new track, the copy stands where the
  old one stands, the copy may leave the playlists out, a master is
  refused, and a node that is not a track is refused.
- **One command** (`sound.track.duplicate`). The sound battery now
  carries **123** commands and the MCP server **334**.
- The **place** of a node in the document is now a helper of the crate
  and not a thing each command writes: the commit (§249) and the
  duplicate stand where the thing they came from stands.

### What it does not settle

- **The copy does not carry the tree's record arm, its input monitor
  and its frozen flag.** The record arm and the input monitor are the
  recording's, and there is no recording here; the frozen flag is a
  thing the tree's window draws, and this tree draws nothing.
- **The copy's name is not made unique**, and a caller that finds a
  track by its name finds the first one with that name.

## 255. The render of a clip

The tree's editor exports the clips as files: it renders the track
over the clip's range and writes what it hears to a WAV file named
after the clip. This tree's battery could render the **mix** and the
**stems**, and not one clip of them.

### What was built

- The command `sound.clip.render` writes the sound of a clip to a file
  in a directory. The file is named after the clip.
- The sound is the sound of the **track** over the clip's range: the
  clips of the track, with their gains and their fades, and not the
  track's trim, not the track's inserts and not the track's fader.
- A **MIDI clip** is refused: it holds notes and not a sound, and a
  file of notes is not a sound.
- A node that is **not a clip** is refused.

### What the port found

- **The tree's export renders the track and this one's renders the
  track too.** The tree's `export_clips` renders the track over the
  clip's range — the clips of the track, with their gains and their
  fades, and without the track's strip — and writes that. It is the
  same sound as the consolidate (§242) and the same sound as the
  commit's file (§249): the sound of the clips and not the sound of
  the track.
- **The tree's export names the file after the clip and makes the name
  unique** — `kick.wav`, and then `kick-2.wav`. Here the names are
  labels and not keys (§252), and the file is named `kick.wav` and a
  second one is `kick-2.wav` because the file is on the disk and two
  files cannot have one name.
- **The tree's export refuses a clip that is too long to render**,
  before it writes anything. Here the render is refused when the
  track's sound is shorter than the clip asks for, which is the same
  kind of refusal and not the same bound.

### What it cost

- **Four tests** in the sound battery (`tests/cliprender.rs`): the
  clip is rendered to a file, the file is named after the clip, a MIDI
  clip is refused, and a node that is not a clip is refused.
- **One command** (`sound.clip.render`). The sound battery now carries
  **124** commands and the MCP server **335**.

### What it does not settle

- **The tree's export may write several clips at once**, and this one's
  writes one: a caller renders the clips one by one.
- **The tree's export writes the clip's own effects** — the tree's
  clip effects are an equaliser and a compressor on one clip — and
  this one's writes the clip's own **processes** and nothing else.

## 256. The taking away of an event of the map

The tree's editor removes a tempo change: the tempo map is a list of
events, and an event may go away as well as come. This tree's battery
could set a tempo, a meter and a key, and could take **none** of them
away: a map that can only grow is not a map a person can work with.

### What was built

- The command `sound.tempo.remove` takes the tempo at a tick away, and
  `sound.meter.remove` takes the meter at a tick away, and
  `sound.key.remove` takes the key at a tick away.
- The event at the **first tick** is **refused**: it is the document's
  tempo, its meter and its key, and a document without one has no beat.
- A tick with **no event** is refused: a tempo that was never there is
  not a tempo that was taken away.

### What the port found

- **The tree has a command for the tempo and not for the meter and not
  for the key.** The tree's `event.tempo_remove` takes a tempo change
  away; the tree's meter and the tree's key are put and never taken
  away. The rule is the same for all three — the event at the first
  tick is the document's own and it stays — and a map that can only
  grow is not a map, so all three are here.
- **The tree's removal says how many it removed**, and answers with
  zero when there was nothing there. Here a tick with no event is
  **refused** and it says why: a command that does nothing and says
  nothing is a command that cannot be told from a command that was
  never run.
- **The tree's removal takes a place and this one's takes a tick.**
  The tree's command takes the place in samples and works out the tick
  itself; the tempo map here is in ticks (§ the music of a MIDI file),
  and a command that takes the map's own unit is a command that does
  not make the caller work out the map.

### What it cost

- **Six tests** in the sound battery (`tests/temporemove.rs`): the
  tempo is taken away, the first tempo stays, a tick with no tempo is
  refused, the meter is taken away, the first meter stays, and the key
  is taken away.
- **Three commands** (`sound.tempo.remove`, `sound.meter.remove` and
  `sound.key.remove`). The sound battery now carries **127** commands
  and the MCP server **338**.

### What it does not settle

- **The tree's `event.tempo_remove` removes a tempo and leaves the
  bars where they are**, and this one's does too: the bars are counted
  from the meter, and a tempo that goes away does not renumber them.
- **The tree's removal of a tempo may leave the map with no tempo at
  the first tick** — it refuses that — and this one's refuses it with
  a word about the document's tempo.

## 257. The shapes the tree makes that this tree did not

The tree's editor makes a **star**, a **line** and a **spiral** out of
nothing but a few numbers, and it makes an **arc** and a **flare** as
well. This tree's vector battery had three shapes — the rectangle, the
ellipse and the polygon — and the star, the line and the spiral are
here now.

### What was built

- The command `vector.star` makes a star: its points alternate
  between an **outer** radius and an **inner** one, it starts at the
  **top**, and the **rotation** turns it. The inner radius is half the
  outer one when it is not given.
- The command `vector.line` makes a line: a contour of **two points**,
  open and not closed.
- The command `vector.spiral` makes a spiral: a run of **quarter
  arcs** that get smaller and smaller, wound **inward** — the path
  starts at the centre and ends at the outside, the way a person draws
  one.
- The geometry is the **tree's own**: the same points, the same
  **kappa**, the same reversal of the contour.
- A star with too few or too many **points** is refused, and a spiral
  whose **decay** is outside its bound is refused.

### What the port found

- **The tree's star takes the absolute value of its radii, and this
  one refuses a negative one.** The tree's `shape.star` takes
  `radius1.abs()` and `radius2.abs()`, so a star asked for with a
  negative radius is a star of the radius it means. Here a negative
  radius is **refused**: the same rule the fader follows (§ the fader
  of a track).
- **The tree's star clamps its point count and this one refuses it.**
  The tree's star has between two and a thousand points, and the count
  outside that is pulled to the edge. Here the count outside the bound
  is refused.
- **The tree's spiral clamps its decay and this one refuses it.** The
  tree's decay is between five and ninety-nine point nine nine
  percent, and the value outside is pulled to the edge. Here it is
  refused, and the same goes for the number of turns.
- **The tree's line is stroked with the current paint and this one's
  is stroked with a colour the caller gives.** The tree's line takes
  the paint that is current, and a line that is not filled is a line
  that is stroked; here a path without a stroke is not drawn at all,
  so the line carries the colour it is stroked with.

### What it cost

- **Seven tests** in the vector battery (`tests/shapes.rs`): the star
  alternates between two radii, the star starts at the top, the inner
  radius is half the outer one, a star outside the bound is refused,
  the line is two points, the spiral winds inward, and a spiral outside
  the bound is refused.
- **Three commands** (`vector.star`, `vector.line` and
  `vector.spiral`). The vector battery now carries **28** commands and
  the MCP server **341**.

### What it does not settle

- **The tree's arc and the tree's flare are not here.** The arc is
  drawn from an anchor, a reference point and two lengths, and the
  flare is a lens flare made of radial gradients in Screen mode. The
  star, the line and the spiral are the shapes whose geometry is a
  handful of numbers; the arc is a drawing tool's worth of geometry
  and the flare is a paint's worth.
- **The tree's star has a straight-armed variant**, where the inner
  radius is worked out from the outer one so that the shoulders line
  up; here the inner radius is what the caller says.

## 258. The closing and the turning of a path

The tree's editor closes a path's open contour and turns a path the
other way round. This tree's vector battery could do neither.

### What was built

- The command `vector.path.close` closes a path's **last open
  contour**: a contour that is open has a loose end, and a loose end is
  a place where the paint stops in the middle of nowhere.
- A path with **no open contour** is refused: there is nothing to
  close.
- The command `vector.path.reverse` turns a path the other way round:
  the contour starts where it ended, and every curve keeps its shape
  and turns the other way — the two control points change places.

### What the port found

- **The tree's reverse is about the way a path *winds*.** The tree's
  `path.reverse` looks at the **signed area** of each contour and
  turns the ones that wind the wrong way, so that a path with a hole
  in it comes out right whichever way it was drawn. Here the command
  turns the path the other way round, always: a path that is turned
  twice is the path it was.
- **The tree's close may set the first anchor's incoming handle**, and
  this one's does not: the handle is a fact about the anchor, and a
  command that closes a contour does not change the curve at its end.
- **The tree's close says which contour it closed**, and this one's
  says that the path is closed and nothing more: the contour is the
  path's, and a path may have several.

### What it cost

- **Four tests** in the vector battery (`tests/turn.rs`): the open
  contour is closed, a path with no open contour is refused, the path
  is turned the other way round, and turning a path twice gives it
  back.
- **Two commands** (`vector.path.close` and `vector.path.reverse`). The
  vector battery now carries **30** commands and the MCP server
  **343**.

### What it does not settle

- **The tree's reverse turns the contours that wind the wrong way**,
  and this one's turns them all: a caller that wants the path wound
  one way says so twice.
- **The tree's path.average and the tree's path.reshape are not
  here**: the average of a set of points and the reshaping of a path
  are two more pieces of the tree's geometry.

## 259. The anchor in the middle of a segment

The tree's editor adds an **anchor point** on a segment: the segment is
split in two, and the two halves are the same curve. This tree's
vector battery had no way to put a point on a curve.

### What was built

- The command `vector.path.anchor.add` splits a segment of a contour
  in two, and the path has one more point to move. The anchor goes in
  at a place along the segment — `t`, from zero at its beginning to one
  at its end.
- The two halves are the **same curve**: the shape of the path does not
  change, and the point where the halves meet is the point the curve
  had at `t`. The split is worked out by **de Casteljau**, which is
  the tree's own way of cutting a curve.
- A segment that does **not** exist is refused, and so is a contour
  that does not exist.
- The geometry is in the **geometry crate** (`craftkit-vector-geom`),
  where the other path operations live.

### What the port found

- **The tree's anchor is an anchor, and here an anchor is a point of
  the contour.** The tree's path carries a list of **anchors** and the
  segments join them; this tree's path carries the segments themselves,
  and the points of the contour are where the segments begin and end.
  Adding an anchor here is therefore the same thing as splitting a
  segment — which is what the tree's own `insert_anchor` does to its
  anchors.
- **The tree's split takes the segment out of the anchors and puts the
  halves in, and this one's splits the segment itself.** The geometry
  is the same: the tree's `subsegment` is de Casteljau, and so is this.
- **The tree's `t` is clamped and this one clamps it too**: the tree's
  `t` is pulled into the range from zero to one, and a place along a
  segment outside that range is a place at the end of the segment.

### What it cost

- **Five tests** in the geometry crate (`tests/split.rs`) and **three**
  in the vector battery (`tests/anchor.rs`): a split curve is the same
  curve, a split line is the same line, a segment that is not there is
  refused, a split at the end is the same curve, the start of the
  contour does not move, the anchor is added, the shape of the path
  does not change, and the segment that is not there is refused.
- **One command** (`vector.path.anchor.add`) and **one function** in
  the geometry crate (`split_segment`). The vector battery now carries
  **31** commands and the MCP server **344**.

### What it does not settle

- **The tree's delete of an anchor is not here.** The tree deletes the
  anchors that are **selected** and keeps the surviving stretches
  apart rather than joining across the deleted ones — and the
  selection is the editor's state, which this tree does not have.
- **The tree's anchor is smooth or a corner, and here it is what the
  split makes it.** The tree's new anchor is a smooth one, with its
  handles on the curve; here the two halves carry the handles the
  split worked out, which is the same curve and not the same handle.

## 260. The key of an object

A PDF object's **dictionary** is the payload of the node, and this
tree's PDF battery could put an object into the document and could
never change one: a payload that cannot be changed is a payload that
was wrong the moment it was written.

### What was built

- The command `pdf.entry.set` sets a key of an object's dictionary:
  the key is **added** when it is not there and **changed** when it is,
  and the order of the dictionary is kept — a key that is there keeps
  its place and a key that is new goes at the end.
- The command `pdf.entry.remove` takes a key away.
- A key that is **not there** is refused: a key that was never there is
  not a key that was taken away.
- A node that is **not an object** is refused, and so is an **empty**
  key.

### What the port found

- **The tree's page commands are the tree's commands, and here the
  dictionary is the door.** The tree's editor sets a page's boxes, its
  rotation and its metadata with a command each; here the object is
  the model and the dictionary is its payload, so one command sets any
  key of any object. The tree's `page.boxes` is this command with the
  key `MediaBox` and the tree's `page.rotate` is this command with
  the key `Rotate` — and the tree's own file is not here, so what a
  page means is the object's own business.
- **The tree's dictionary keeps its order and so does this one.** The
  port found that a JSON object does not keep the order of its keys
  (§27), and the dictionary is a **list of pairs** — so a key that is
  changed keeps its place and a rewritten file is byte-stable.
- **The tree removes a key by removing the thing that holds it** — a
  page's rotation goes when the page goes — and here a key is taken
  away on its own: the object model has no page to lose.

### What it cost

- **Six tests** in the PDF battery (`tests/entries.rs`): the key is
  added, the key is changed, the key is taken away, a key that is not
  there is refused, a node that is not an object is refused, and an
  empty key is refused.
- **Two commands** (`pdf.entry.set` and `pdf.entry.remove`). The PDF
  battery now carries **20** commands and the MCP server **348**.

### What it does not settle

- **The tree's page is a page and here a page is an object.** The tree
  knows that an object with the type `Page` is a page and gives it
  boxes, a rotation and a tree of children; here the type is a key in
  the dictionary and the rest is the caller's business.
- **The tree's file is not here**: the cross-reference table, the
  incremental save and the parser are the file (§27), and a key that
  is set here is a key in the object model and not yet in a file.

---

## 261. The taking away of an object

A PDF file is a set of **indirect objects**, and this tree's PDF battery
could put one in and never take one out: a file whose objects only grow
is not a file that can be edited.

### What was built

- The command `pdf.object.remove` takes an indirect object away — a
  dictionary or a stream.
- The **references** to it go with it. The edges of the kind `ref` die
  with the node (the SDK does that), and the **values in the
  dictionaries** are taken away as well, because the file is written
  from the dictionaries and not from the edges: a key that held one
  reference loses the key, and a key that held a **list** loses the one
  element that named the object and keeps the rest.
- The **root** of the file — the object whose type is `Catalog` — is
  **refused**: a file without a root is not a file.
- A node that is **not an object** is refused, and so is a node that is
  **not there**.
- The two commands that changed a dictionary's key are **renamed**:
  `pdf.object.set` is `pdf.entry.set` and `pdf.object.remove` is
  `pdf.entry.remove`, because `pdf.object.remove` is now the name of
  the other thing. A dictionary is a list of **entries**, and the code
  has always called them that.

### What the port found

- **The tree's removal is a rebuild, and this one is a surgery.** The
  tree's `delete_pages` walks the page tree, keeps the pages that
  stay, and **rebuilds** the tree around them — flattening the
  attributes the survivors inherited onto the survivors themselves. Here
  there is no page tree to rebuild: the objects are a graph, so what
  has to happen is that the **dictionaries of the other objects** are
  edited, in place, one key at a time. The tree's act is one rebuild;
  this one is N small edits, and the difference is the difference
  between a tree and a graph.
- **The tree's rule is about the last page, and this one is about the
  root.** The tree refuses to delete every page
  (`WouldRemoveAllPages`), because a file with no pages is not a file.
  Here the file needs a root, and the root is refused for the same
  reason: the file must have one thing it cannot do without.
- **The tree drops the destinations that pointed at the page that
  went** — a bookmark and a link lose their `/Dest` and their `/A`
  rather than pointing at nothing — and so does this one: the value in
  the dictionary is taken away, and what pointed at it points at
  nothing, which is what the tree also does.
- **The tree frees the number of the object that went, and this one
  does not.** The tree's `doc.free` puts the number back in the pool;
  here the number is the caller's to give, and `pdf.object.add` takes
  any number at all, so a number that is free again may be given to
  another object. That is a gap, and §262 closes half of it: a number
  may not be taken twice, and a number that is free again may be given
  to another object.

### What it cost

- **Six tests** in the PDF battery (`tests/remove.rs`): the object is
  gone, the references to it go with it, a list loses the one element
  that named it, the root is refused, a node that is not an object is
  refused, and a node that is not there is refused.
- **One command** (`pdf.object.remove`) and **two renames**. The PDF
  battery now carries **20** commands and the MCP server **348**.

### What it does not settle

- **The bytes of a stream stay.** The bytes live in a file beside the
  document, and taking the object away does not take the file away: an
  orphaned file is left behind.
- **What the object held stays.** An annotation of a page that is taken
  away is an object of its own, and it stays in the document — an
  object that nothing refers to any more. The tree drops the widgets
  of a page that goes; here they are orphans, and nothing gathers
  them.
- **The tree's file is not here**: the cross-reference table is
  written by the writer from the objects that are in the document, and
  an object that is gone is not in the table.

---

## 262. The number of an object

An indirect object is named by its **number** in the file, and every
reference to it is that number. The battery let two objects share one
number, and a file with two objects of one number is a file that cannot
say which one it means.

### What was built

- The command `pdf.object.number` changes the number of an object, and
  **every reference to it follows**: the references in the dictionaries
  of the objects that point at it are changed, the ones in a list one
  element at a time, and the ones inside a nested dictionary as well.
- The commands `pdf.object.add` and `pdf.stream.add` **refuse a number
  that is taken**: one number is one object.
- A number that is **another object's** is refused, and so is a number
  the object **already has**, and so is a number that is **not a whole
  number**, and so is zero.

### What the port found

- **The tree hands the numbers out, and here the caller does.** The
  tree's `doc.free` puts the number of a deleted object back in the
  pool, and a new object takes a free one — the tree is the one that
  decides, so the tree never has to check. Here the number is a
  **parameter**: `pdf.object.add` takes a number and not "the next
  one", so "one number is one object" is a rule that has to be
  **enforced** rather than assumed. The tree's rule is a fact about the
  tree; this one is a check.
- **A number is a fact with two faces.** In the tree a reference names
  an `ObjRef` — a number and a generation — and the file's table names
  the same pair. Here the number is in the node's payload and in the
  dictionaries of the other objects, and the two have to agree, so a
  number that is changed is a **surgery on every other object** and
  not a field that is set. The tree never changes a number, so it never
  has to make the references follow.
- **The tree's pages keep their numbers when the tree is rebuilt.**
  `delete_pages` rebuilds the page tree around the pages that stay and
  the survivors keep their `ObjRef`s. Here the same act is not a
  rebuild but a change of one number and of every reference to it, and
  a number may be changed at will — a thing the tree does not offer.

### What it cost

- **Seven tests** in the PDF battery (`tests/number.rs`): a number that
  is taken is refused, the number is changed and the references follow,
  a number that is another object's is refused, a number the object
  already has is refused, a number that is not a whole number is
  refused, a node that is not an object is refused, and a node that is
  not there is refused.
- The **tests that added two streams to one session** were given free
  numbers: one number is one object.
- **One command** (`pdf.object.number`). The PDF battery now carries
  **20** commands and the MCP server **348**.

### What it does not settle

- **The generation is not moved.** A reference names a number and a
  generation, and the generation says which version of an object is
  meant; this arc moves the number and keeps the generation, because
  an object here has no generation of its own.
- **A number that is free may be given away.** Nothing in the tree
  stops that either — the tree's pool does it deliberately — but here
  the caller is the one who decides, and the tree's `doc.free` has no
  counterpart that says which numbers are free.
- **The tree's file is not here**: the cross-reference table is written
  from the objects that are in the document, and because a number may
  not be taken twice, the table has one entry for every number.

---

## 263. The trimming of a clip

A clip has **two clocks** — where it starts on the timeline and where it
begins in the media — and two **edges**, and the film battery could put
a clip on a track and never move one of them.

### What was built

- The command `film.clip.trim` trims one edge of a clip. The **in**
  edge is the head: the clip gets shorter or longer, the media it
  begins at moves, and the clip moves on the timeline with it. The
  **out** edge is the tail: the clip gets shorter or longer and stays
  where it is.
- A trim that would cross a bound is **refused**, and the refusal says
  which bound stopped it. The bounds are four: a clip is **at least one
  frame** long, a clip may not run **past the end** of the media or
  begin **before** it, a clip may not **overlap** the clip before it or
  the clip after it on the same track, and a clip may not start
  **before the timeline**.
- A delta that is **not a whole number** is refused, and so is a delta
  of **zero frames** — a trim of nothing is not a trim — and so is an
  edge that is not `in` or `out`, and so is a node that is not a clip.

### What the port found

- **The tree clamps, and this tree refuses.** The tree's
  `clamp_trim` computes the biggest delta the trim may have — bounded
  by the previous clip, the next clip, the length of the media and the
  minimum duration — and `trim` uses it and gives back the delta it
  used. So a drag that goes too far stops at the bound, and **the delta
  that happened is not the delta that was asked for**: the difference is
  there, in the number the tree gives back, and it is the caller's to
  notice. Here the same drag is an **error that names the bound**:
  "the clip would end at frame 40, and the clip after it starts at
  frame 40". The tree's clamp answers with a different number, and this
  tree's refusal answers with the reason.
- **The tree's trim has two modes and this one has one.** The tree's
  `TrimMode::Regular` moves the clip's head and its place on the
  timeline together, and its `TrimMode::Ripple` moves the head and
  **closes the gap** by shifting every clip after it. Here the trim is
  the regular one: the clip moves and the gap it leaves is a gap. The
  ripple is a different thing, and it is a different command.
- **The tree's trim counts the media in the media's own ticks.** The
  tree's clips have a speed, and a trim of ten frames of the timeline
  is ten times the speed of frames of the media, so `src_of` multiplies
  the delta by the speed before it touches the source. Here a clip has
  no speed: a frame is a frame in both clocks, and the two clocks are
  the same number of frames apart.
- **The tree's neighbours are the clips on the same track, and here
  they are found the same way** — but the tree knows the track of a
  clip because a clip *is* in a track, and here a clip is in a track as
  well: the containment is the same, and the tree of the document is
  the tree of the editor.

### What it cost

- **Twelve tests** in the film battery (`tests/trim.rs`): the in edge
  is trimmed, the out edge is trimmed, a clip that would be too short
  is refused, a clip that would run past the media is refused, a clip
  that would begin before the media is refused, a clip that would
  overlap the one before it is refused, a clip that would overlap the
  one after it is refused, a node that is not a clip is refused, an
  edge that is not an edge is refused, a delta that is not a whole
  number is refused, a trim of zero frames is refused, and a node that
  is not there is refused.
- **One command** (`film.clip.trim`). The film battery now carries **6**
  commands and the MCP server **349**.

### What it does not settle

- **The ripple is not here.** The tree's ripple trim closes the gap,
  and a ripple is a different command and not a flag.
- **The moving of a clip is not here — §264 is where it came.** The
  tree's `move_items` has two modes, the **overwrite** and the
  **insert**; the plain move is in §264, and the overwrite is not,
  because a command that cuts a clip without saying so is not a
  command.
- **The tree's linked clips are not here.** The tree's clips may be
  **linked**: one clip moves and its partners move with it. Here a clip
  is a clip.

---

## 264. The moving of a clip

A clip has a **place** on a track, and the film battery could put a
clip on a track and never move it.

### What was built

- The command `film.clip.move` moves a clip **along its track** or
  **to another track**. A clip that changes track is taken out of the
  tree and put under the other track **with the same id**, so that the
  edge that says which item it plays still says it.
- A move that would **land on another clip** is **refused**, and the
  refusal names the clip that would have been cut: "the clip would run
  from frame 110 to frame 130, and the clip on the track runs from
  frame 100 to frame 120". A move that only **touches** the end of the
  other clip is allowed.
- A start that is **before the timeline** is refused, and so is a start
  that is **not a whole number**, and so is a node that is not a clip,
  and so is a node that is not a track, and so is a node that is not
  there.

### What the port found

- **The tree's move has two modes and one of them destroys.** The
  tree's `move_items` takes an `insert_mode`: the **overwrite** cuts
  whatever is under the clip that moves, and the **insert** pushes the
  clips along the track. The overwrite is a thing a command does
  **quietly** — the answer the tree gives back is the answer to "did
  the move happen", and not to "what did the move destroy". Here the
  plain move is the only move, and a move that would land on another
  clip is a **refusal that names the clip**: the caller may then cut it
  in a command of its own, and the two commands are two things that
  happened.
- **The tree's move is a move between tracks, and the tree's track is
  a list of clips.** The tree's `Track` holds `items` and the move
  puts the clip in another track's list. Here the track holds the
  clips as **children**, and a clip that changes track is taken out of
  the tree and put under the other one — and the **id is kept**,
  because the id is what the edge from the clip to the item names, and
  an edge that names a clip that is not there is a dangling edge.
- **The tree's clips may be linked, and here they are not.** The
  tree's `TrackItem` has a `link`, and a move moves the linked clips
  together. Here a clip is a clip, and the link is not here: the
  relation that says which item a clip plays is the only edge a clip
  has.

### What it cost

- **Nine tests** in the film battery (`tests/move.rs`): a clip moves
  along its track, a clip moves to another track, a move that would
  land on another clip is refused, a move that touches the end of
  another clip is allowed, a start before the timeline is refused, a
  start that is not a whole number is refused, a node that is not a
  clip is refused, a node that is not a track is refused, and a node
  that is not there is refused.
- **One command** (`film.clip.move`). The film battery now carries **7**
  commands and the MCP server **350**.

### What it does not settle

- **The insert was not here — §266 is where it came.** The tree's
  insert mode pushes the clips after the moved one along the track, so
  that nothing is cut; it is a different command and not a flag.
- **The tree's overwrite is not here**, and nothing here cuts a clip
  that a move would land on.
- **The tree's linked clips are not here**, and neither is the tree's
  **locked** track: a locked track refuses a move in the tree, and
  here a track has no lock to refuse with.

---

## 265. The keyframes of a property

The compositing battery could set a property and read it, and could
not put a **keyframe** on it — and the battery's own finding says that
the value at a time is derived, so there was no way to make one.

### What was built

- The command `fx.keyframe.set` sets a keyframe: the keyframe that is
  there is **changed**, the one that is not there is **added**, and the
  list stays **sorted by frame**.
- The command `fx.keyframe.remove` takes a keyframe away, and a frame
  that has **no keyframe** is refused: a keyframe that was never there
  is not a keyframe that was taken away.
- The command `fx.prop.at` reads the **value at a frame**: the first
  keyframe's value before the first keyframe, the last keyframe's value
  after the last one, and the **line** between them in between.
- The **interpolation** is the tree's: a number is interpolated
  between two numbers, a list of numbers between two lists of the same
  length, and anything else is **held** — the value of the earlier
  keyframe.
- A property that is **not animated** cannot have a keyframe: it is
  refused, and it says so. A path that names **nothing** is refused,
  and so is a frame that is **not a whole number**, and so is a node
  that is not a layer.

### What the port found

- **A keyframe in the tree is more than a frame and a value.** The
  tree's `Keyframe` carries an interpolation on **each side** —
  linear, bezier or hold — and an ease with a speed and an influence,
  and the handles of a spatial curve, and a label. Here a keyframe is
  a frame and a value, and the curve is the line: the tree's `Interp`
  has three values and one of them is here.
- **The tree keeps the shape of a keyframe when only its value
  changes.** The tree's `set_key` replaces a key at the same time and
  **keeps its interpolation, its eases, its handles and its label** —
  the curve is the keyframe's, and the value is a thing that comes and
  goes. Here there is no curve to keep, and the keyframe is the frame
  and the value, so changing the value changes the keyframe.
- **The hold is a property of the value and not only of the curve.**
  The tree's `segment_value` holds the earlier value when
  `a.value.interpolates()` is false — a colour or a string does not
  interpolate — and not only when the keyframe says "hold". Here the
  same rule: two numbers and two lists of numbers are interpolated,
  and anything else is held, and the difference is not asked about.
- **The tree's value at a time is a function of the keyframes and of
  nothing else.** The tree's `evaluate` finds the segment with a
  partition point and the fraction `x` between the two keys, clamped
  to the segment; before the first key it is the first key's value and
  after the last key it is the last key's. Here the same function,
  with the keyframes in the payload, because a property tree is not a
  tree of nodes (§31).

### What it cost

- **Thirteen tests** in the fx battery (`tests/keys.rs`): a keyframe
  is set, a keyframe that is there is changed, the value in between is
  the line, the value outside the keyframes is held, a list of numbers
  is interpolated, a value that does not interpolate is held, a
  property that is not animated has one value, a keyframe is taken
  away, a frame that has no keyframe is refused, a property that is
  not animated is refused, a path that names nothing is refused, a
  frame that is not a whole number is refused, and a node that is not
  a layer is refused.
- **Three commands** (`fx.keyframe.set`, `fx.keyframe.remove` and
  `fx.prop.at`). The fx battery now carries **7** commands and the MCP
  server **353**.

### What it does not settle

- **The shape of the curve is not here.** The tree's bezier eases, its
  spatial handles and its `Hold` interpolation are not here: the value
  between two keyframes is the line.
- **The expression is not here.** The tree may put an expression on a
  property, and an expression is a language and not a value.
- **The tree's linked properties are not here**: the tree may key two
  properties together, and here a property is a property.

---

## 266. The inserting of a clip

The tree moves a clip in two modes: the **overwrite**, which cuts what
is under the clip, and the **insert**, which opens a gap and pushes the
clips along. The overwrite is §264, and this is the insert.

### What was built

- The command `film.clip.insert` puts a clip at a place on a track and
  opens a **gap of the clip's length** there: every clip on the track
  that starts at or after that place is **pushed along** by that
  length, so that nothing is cut. The clip may go to **another track**,
  and the gap opens there.
- A clip that **spans** the place is **refused**, and the refusal names
  it: "the clip on the track runs from frame 20 to frame 50, and it
  spans the place the gap opens at". The tree splits such a clip so
  that the gap has two edges; here splitting is a command of its own.
- A start that is **before the timeline** is refused, and so is a start
  that is **not a whole number**, and so is a node that is not a clip,
  and so is a node that is not a track, and so is a node that is not
  there.

### What the port found

- **The tree's insert opens the gap and then places the clip, and the
  order matters.** The tree's `insert` computes the place and the
  length of the gap from the placements, opens the gap on every
  affected track, and only then puts the clips down. Here the gap is
  the clip's own length and the place is the one the caller gave, so
  the order is the same: the gap opens, the clips move, and the clip
  goes down.
- **The tree's gap splits a clip that spans the place, and that split
  is a command in its own right.** The tree's `split_track_at` cuts a
  clip in two: the left piece keeps its id, the right piece gets a new
  one, and the media moves with the cut — and the tree's razor does
  the same thing on purpose. Here a split is not a side effect: the
  insert is **refused** and the clip that spans the place is named,
  because a command that splits a clip while it moves another one is a
  command that does two things and says one.
- **The tree shifts the clips that start at or after the place, and
  not the ones that span it.** The tree's `shift_track_from` moves
  every item whose start is at or after the place; a clip that spans
  the place is handled by the split that comes before it. Here the
  clips that start at or after the place are pushed, and a clip that
  spans it is refused, so the two cases do not overlap.
- **The tree's insert opens the gap on the sync-locked tracks too.**
  The tree's `insert` walks the tracks and opens the gap on the ones
  that are **sync-locked** with the target, so that a group of
  tracks stays together. Here a track is a track, and there is no
  lock to be in.

### What it cost

- **Ten tests** in the film battery (`tests/insert.rs`): an insert
  pushes the clips along, the gap opens at the place, a clip that
  spans the place is refused, a clip is inserted on another track, the
  end of the track grows, a start before the timeline is refused, a
  start that is not a whole number is refused, a node that is not a
  clip is refused, a node that is not a track is refused, and a node
  that is not there is refused.
- **One command** (`film.clip.insert`). The film battery now carries
  **8** commands and the MCP server **354**.

### What it does not settle

- **The sync-locked tracks are not here**, and neither is the tree's
  **locked** track: a locked track refuses an edit in the tree, and
  here a track has no lock to refuse with.
- **The split was not here — §267 is where it came.** The tree's razor
  divides a clip at a place, and that is a command of its own — the
  same command that would make the insert possible on a clip that
  spans the place.

---

## 267. The splitting of a clip

The tree's razor divides a clip at a place, and the tree's insert
splits a clip that spans the place as a side effect. Here the split is
a **command of its own**, and the insert refuses to split anything.

### What was built

- The command `film.clip.split` divides a clip at a place: the piece
  **before** the place keeps the clip's id and is shortened, and the
  piece **after** it is a **new clip** that begins that much later in
  the media.
- The new clip **plays the same item** and is **on the same track**.
- A place that is **not inside** the clip is refused: a cut at the edge
  would cut nothing, and a place outside the clip is not in it. A
  place that is **not a whole number** is refused, and so is a node
  that is not a clip, and so is a node that is not there.

### What the port found

- **Cutting a clip cuts both of its clocks.** The tree's
  `split_track_at` shortens the left piece and gives the right piece a
  **new id**, and it moves the right piece's `source_in` by the length
  of the cut: the piece after the cut shows the media that comes after
  the cut, not the media the cut was at. Here the same: the second
  piece's `source_in` is the first piece's plus the cut. A clip has
  two clocks and a cut is in both of them.
- **The tree's split is a helper, and the tree's razor is the
  command.** The tree's `split_track_at` is called by the insert, by
  the overwrite and by the delete, and the razor calls it to divide a
  clip on purpose. Here the split is a command, and the insert is
  **refused** when a clip spans the place — a command that splits a
  clip while it moves another one is a command that does two things and
  says one.
- **The tree's split keeps the links together and drops the
  transitions.** The tree's split gives the new piece a **new link**,
  so that the two pieces are still linked to each other and not to the
  old partner, and it removes the transitions that spanned the cut and
  re-points the ones after it. Here the links and the transitions are
  not here: a clip is a clip, and a new clip plays the item the old
  one played.

### What it cost

- **Six tests** in the film battery (`tests/split.rs`): a clip is
  split, the new clip plays the same item, a place that is not inside
  the clip is refused, a place that is not a whole number is refused, a
  node that is not a clip is refused, and a node that is not there is
  refused.
- **One command** (`film.clip.split`). The film battery now carries
  **9** commands and the MCP server **355**.

### What it does not settle

- **The tree's razor is not here**: the tree divides a clip on several
  tracks at once, and here a split is one clip.
- **The tree's links and transitions are not here**, and neither is the
  tree's **reversed** clip, whose cut moves the media the other way.

---

## 268. The taking away of a clip

The film battery could put a clip on a track and could never take one
away.

### What was built

- The command `film.clip.remove` takes a clip off its track.
- What goes with it goes with it: the edge that says which item the
  clip played **dies with the node**.
- The gap the clip leaves is a **gap**: the clips that were after it
  stay where they are, and the track is shorter by the length of the
  clip that went.
- A node that is **not a clip** is refused, and so is a node that is
  **not there**.

### What the port found

- **The tree has two ways of taking a clip away, and they are not the
  same.** The tree's `delete_items` removes the clips and leaves the
  gap; the tree's `ripple_delete_items` removes them and **closes** the
  gap, and it **refuses** when it cannot: "no gap can close: the other
  clips on these tracks cover the deleted time". Here the delete is
  here, and the ripple is a command of its own: a command that removes
  a clip and closes a gap does two things, and the gap that closes is
  the tree's business.
- **What goes with a clip goes with it, and the core says so.** The
  SDK's `remove_node` removes the relations that name the node and the
  relations the node names, so the edge from the clip to the item dies
  with the clip — and there is nothing left that points at nothing.
  The tree has the same rule in its own words: the tree's
  `delete_items` removes the clips that are named and drops the
  **transitions** that are left without a clip on one side
  (`remove_orphan_transitions`).
- **The gap is a fact about the timeline and not about the tree.** The
  track's end is the end of its last clip (§ the film battery's own
  finding), so a clip that is taken away makes the track shorter
  without anybody writing that down.

### What it cost

- **Five tests** in the film battery (`tests/remove.rs`): a clip is
  taken away, the edge goes with the clip, the gap is left, a node
  that is not a clip is refused, and a node that is not there is
  refused.
- **One command** (`film.clip.remove`). The film battery now carries
  **10** commands and the MCP server **356**.

### What it does not settle

- **The ripple delete was not here — §269 is where it came.** The
  tree's **lift** and its **extract**, which take a **range** away
  rather than a clip, are still not here.
- **The tree's linked clips are not here**: a link is a tree's way of
  saying that two clips move together, and here a clip is a clip.

---

## 269. The rippling of a clip

The tree's delete leaves a gap and the tree's **ripple delete** closes
it. The delete is §268, and this is the ripple.

### What was built

- The command `film.clip.ripple` takes a clip away and **closes the
  gap**: the clips that were after it move **up** by the length of the
  clip that went, and the time the clip took is free again. The clip
  before it does not move.
- A ripple whose gap **cannot close** is **refused**, in the tree's own
  words: "no gap can close: the clip on the track runs from frame 10 to
  frame 20, and it covers the time the clip took".
- A node that is **not a clip** is refused, and so is a node that is
  **not there**.

### What the port found

- **The tree's ripple computes the stretches that can close, and closes
  each of them.** The tree's `ripple_delete_spans` merges the time the
  deleted clips covered with the time that stays, and the stretches
  between them are the gaps that may close; the tree then closes them
  one at a time, right to left. Here the stretch is the whole range of
  the clip or nothing: the clip's range is free or it is not.
- **The tree's refusal is a fact about the material, and here it is
  too.** The tree refuses when the deleted time is covered by what
  stays on the tracks that lose a clip: "no gap can close: the other
  clips on these tracks cover the deleted time". Here the clip that
  covers the time is named, and the ripple is refused.
- **The tree's ripple moves the sync-locked tracks too.** The tree
  shifts the tracks that are **sync-locked** with the track that lost
  the clip, so that a group of tracks stays together — and it refuses
  when a locked track would have to move into time that is taken. Here
  a track is a track, and there is no lock to be in.
- **The tree's ripple closes the gaps right to left, and here the
  clips are moved once.** The tree closes one stretch at a time, from
  the last to the first, because a stretch may depend on the ones
  before it. Here the clip's range is one stretch, and the clips that
  are after it move once.

### What it cost

- **Five tests** in the film battery (`tests/ripple.rs`): the gap
  closes, the clip before does not move, a gap that cannot close is
  refused, a node that is not a clip is refused, and a node that is
  not there is refused.
- **One command** (`film.clip.ripple`). The film battery now carries
  **11** commands and the MCP server **357**.

### What it does not settle

- **The sync-locked tracks are not here**, and neither is the tree's
  **locked** track.
- **The tree's lift was not here — §272 is where it came.** The
  tree's **extract** is still not here: it takes a range away and
  closes the gap, where the lift leaves it.

---

## 270. The filling of a range

The spreadsheet battery could set a cell and could not fill a **range**.

### What was built

- The command `grid.range.fill` fills a range from a cell. The tree has
  two shapes for it — the plain drag **copies** and the series
  **counts on** — so the fill has a **mode**, and it says which:
  `copy` and `series`. The **default is the series**, because the
  tree's command defaults to the series too.
- The patterns of the series are the tree's: a **number** counts on by
  one, and **text with a number in it** — `Item 7`, `Q003x` — counts
  its number on and keeps the **width** of the digits. Anything else is
  **copied**.
- The **target holds the source at a corner**: a fill from `A1` into
  `A1:A10` goes **down**, a fill from `A10` goes **up**, and a fill
  into `A1:C1` goes **right**. A target that does not hold the source
  is refused, and so is a source that is not at a corner.
- The fill is **bounded**: the tree refuses a fill whose source or
  target is bigger than five million cells — "range too large" — and
  this one refuses a fill that would write more than **ten thousand**
  cells.
- A source that holds a **formula** is refused: the tree copies the
  formula and shifts its references, and that is a thing of its own.
  A source that is **empty** is refused, and so is a mode that is not a
  mode, and so is a sheet that is not there.

### What the port found

- **The tree has four fills and they differ in one cell.** The tree's
  plain drag of a lone number **copies** it — a drag of `1` gives
  `1, 1, 1` — and so does the tree's **series**, which is the tree's
  *default*: it is the tree's right-drag **Fill Series** (`extend`) and
  its `Ctrl`+drag (`alternate`) that count a lone number up. So "fill"
  is four different things in the tree, and a command that picked one
  of them silently would be wrong most of the time. Here the fill
  **says** which one it is — and the tree's own command says too:
  `edit.autoFill` takes a mode, and there are nine of them (`copy`,
  `alternate`, `formats`, `values`, `extend`, `days`, `weekdays`,
  `months`, `years`), and **the series is the default**.
  **§270 said the tree's series counted a lone number up, and that was
  wrong: the series copies it, and the `extend` counts it up.** The
  correction is in §274, where the modes are the tree's.
- **The tree's corner rule is the shape of the operation.** The tree's
  `holds_source_in_corner` says that a target must contain the source
  and that the source must sit in one of its corners, and the tree
  refuses a fill that does not: "the target must contain the source in
  one of its corners". A fill has a **direction**, and the direction
  comes from where the source is in the target.
- **The tree's series is a pattern, and the patterns are read off the
  source.** The tree's `Pattern::detect` finds a numeric series, a
  text with a number in it, or a list of days and months, and the
  pattern decides what every cell of the range holds. Here the numeric
  series and the text with a number are here, and the **lists** — the
  days, the months, the quarters — are not: a text that is a day of
  the week is copied here, and the tree would step through the week.
- **The tree's numbers are rounded to fifteen digits.** The tree's
  `round15` keeps a series of numbers from growing a tail of floating
  point noise, and here the same: `0.1` plus `0.2` counts on as
  `0.3` and not as `0.30000000000000004`.
- **The tree's overflow is a nothing and not a wrap.** The tree's
  series near the largest number yields nothing: the cell keeps what
  it had, rather than wrapping around. Here the same: a cell whose
  number would overflow is not written.

### What it cost

- **Thirteen tests** in the grid battery (`tests/fill.rs`): a copy
  fill copies the value, a series fill counts a number on, a series
  fill counts the number in a text on, a text with no number is copied,
  the fill goes up, the fill goes right, a target that does not hold
  the source is refused, a source that is not at a corner is refused, a
  source that holds a formula is refused, an empty source is refused, a
  mode that is not a mode is refused, a fill that is too large is
  refused, and a sheet that is not there is refused.
- **One command** (`grid.range.fill`). The grid battery now carries
  **9** commands and the MCP server **358**.

### What it does not settle

- **A formula is not filled** — and that is no longer true: a formula
  is filled, and its references are shifted (§275).
- **The tree's dates and lists are not here** — and the **dates** are
  here now (§276), and the **lists** are not: the tree's **quarter**
  (`Q1`, `Q2`, `Q3`, `Q4`) and its **ordinal** (`1st`, `2nd`) are not
  here either.
- **The tree's source may be a range.** The tree fills lane by lane
  from a source that is a range of cells; here the source is one cell.

---

## 271. The number format of a cell

The spreadsheet battery could set a cell and could not say how the cell
is **written**.

### What was built

- The command `grid.cell.format` puts a **number format** on a cell.
  The number formats this tree has are `general`, `0`, `0.00`, `#,##0`,
  `#,##0.00`, `0%` and `0.00%` — and they are a **grammar**, so
  `0.0000` and `#0` are codes too. A format that is **not a number
  format of that shape** is refused, and so is a format that is **too
  long** — the tree's own bound of 255 characters.
- The command `grid.cell.text` reads the **text the cell shows**: the
  value of the cell written with its format. The value of a **formula**
  cell is computed, so this is the calculation.
- A **percentage** is multiplied by a hundred, the **thousands** are
  separated, and a number is **rounded away from zero** — the tree's
  rounding.
- A cell may be formatted **before it has a value**, setting a value
  or a formula in a cell keeps its **format**, and the **fill** copies
  the format of the source.
- A **text** is not changed by a format, and a cell that is **not
  there** — a sheet that is not there — is refused.

### What the port found

- **The tree's format is a style, and a style is shared.** In the tree
  a cell has a **style** and the style holds the format, so a hundred
  cells with one format carry one format and changing the style changes
  them all. Here the format is a **field of the cell**, so a hundred
  cells with one format carry a hundred formats — and the tree's style
  also carries a font, a fill, borders, an alignment and a protection,
  none of which are here. The port found the same bargain the compositing battery
  made with its paths: the thing that is too many to be a node goes in
  the payload, and the plugin owns it.
- **The tree's format is a language and this one is a subset.** The
  tree's `NumberFormat` parses sections, colours, fills, dates, times,
  currency, accounting, fractions, scientific notation and text
  sections; and it **never fails** — an unknown part of a code becomes
  literal text, and a code that is too long becomes General, with the
  code kept as it is. Here a code that is neither a number format of
  this shape nor one of the nine dates is **refused**: the tree
  degrades quietly, and a format that is quietly ignored is a format
  the user believes in and the program does not have.
- **The number formats of this tree are a grammar, and the date
  formats are a list — and §271 said that both were lists.** It said
  "a code that is not one of them is refused", and named seven. The
  seven are the ones the tree's own tests use; the **code** takes any
  number of decimals and any arrangement of `0` and `#`, so `0.0000`
  and `#0` are formats this tree has had since §271. The truth is that
  the number formats are a small **grammar** — the thousands, a whole
  part of placeholders with at least one `0` in it, up to fifteen
  decimals and a percent sign — and that the date formats are a **list**
  of nine, because the tree's date formats are a language and this one
  has nine shapes of it (§276). The claim is corrected here, and the
  crate's own words with it.
- **The tree's General format is about a screen.** The tree's General
  format fits the number to the **width of the column** — fewer
  decimals, then scientific notation, then `####` — because a cell is
  drawn in a column that is not wide enough. Here the General format is
  the number as it is: a document is not a screen, and the width of a
  column is not in it.
- **The tree's rounding is away from zero, and it is not Rust's
  default.** The tree's numbers are written with the digits the format
  asks for, rounded the way a spreadsheet rounds: `0.125` with two
  decimals is `0.13`. Rust's own formatting rounds halves to even and
  would write `0.12`, so here the number is scaled and **rounded away
  from zero** before it is written.

### What it cost

- **Twelve tests** in the grid battery (`tests/format.rs`): a number is
  written with its decimals, the thousands are separated, a percentage
  is multiplied by a hundred, a number is rounded away from zero, a
  cell with no format shows its value, a text is not changed by a
  format, a formula cell shows what it computes, a format that is not
  one of ours is refused, a format that is too long is refused, a cell
  may be formatted before it has a value, the fill copies the format,
  and a sheet that is not there is refused.
- **Two commands** (`grid.cell.format` and `grid.cell.text`) and a
  **field** on the cell: the format. The grid battery now carries **11**
  commands and the MCP server **360**.

### What it does not settle

- **The tree's times, currency, accounting, fractions and scientific
  formats are not here**, and neither are the tree's **sections** — a
  format with one code for positive numbers and another for negative
  ones is not here. The tree's **dates** are here now, and they are a
  list of nine shapes (§276).
- **The tree's style is not here**: a style is shared between cells and
  carries a font, a fill, borders, an alignment and a protection, and
  here the format is the cell's own.
- **The tree's `TEXT` function is not here** — and it is now (§278):
  the tree's `TEXT` writes a value with a format code inside a
  formula, and a formula in this tree has the codes of the format.

---

## 272. The lifting of a range

The tree has two ways of taking a **range** of a track away: the
**lift**, which leaves the gap, and the **extract**, which closes it.
The lift is here.

### What was built

- The command `film.clip.lift` takes the clips that are **in a range**
  of a track away, and the gap they leave is a **gap**: the clips that
  were after the range stay where they are, and the track is shorter by
  the length of the range.
- A clip that **spans an edge** of the range is **refused**, and the
  refusal names it: the tree splits it, and the split is a command of
  its own.
- A range that **ends before it starts** is refused, and so is a range
  that is **not whole frames**, and so is a node that is not a track,
  and so is a node that is not there.

### What the port found

- **The tree's lift is a remove and a split.** The tree's `lift` calls
  `clear_track_range`, and that **splits the clips at the boundaries
  of the range** and then removes the clips that are inside it — so the
  range has two edges, and the material that was in the range is gone
  and the material around it is still there. Here the range has to
  have edges that exist: a clip that spans an edge is refused, and the
  split that would make the edge is a command of its own.
- **The tree's two ways of taking a range away differ in one thing.**
  The tree's `lift` removes the material and leaves the gap; the
  tree's `extract` removes the material **and closes the gap** by
  shifting the clips that were after the range up by the length of the
  range. So the tree's lift and this lift are the same operation, and
  the tree's extract is the lift plus one more thing — which is why
  they are two commands here too.
- **The tree's lift works on several tracks at once, and the extract
  works on the sync-locked ones as well.** The tree's `lift` takes a
  list of tracks; the tree's `extract` also closes the gap on the
  tracks that are **sync-locked** with the target, so that a group of
  tracks stays together. Here a track is a track, and there is no lock
  to be in.

### What it cost

- **Seven tests** in the film battery (`tests/lift.rs`): the clips in
  the range are taken away, the clips outside the range stay where they
  are, a clip that spans an edge is refused, a range that ends before
  it starts is refused, a range that is not whole frames is refused, a
  node that is not a track is refused, and a node that is not there is
  refused.
- **One command** (`film.clip.lift`). The film battery now carries
  **12** commands and the MCP server **361**.

### What it does not settle

- **The extract was not here — §273 is where it came.** The tree's
  **sync-locked** tracks and its **locked** tracks are still not here.
- **The tree's links and transitions are not here**, and neither is the
  tree's **caption tracks**, which the extract also closes the gap on.

---

## 273. The extracting of a range

The tree's other way of taking a range of a track away is the
**extract**: the same removal, and the gap **closes**.

### What was built

- The command `film.clip.extract` takes the clips that are **in a range**
  of a track away **and closes the gap**: the clips that were after
  the range move **up** by the length of the range, so the time the
  range took is time that is free again.
- A clip that **spans an edge** of the range is refused, as it is for
  the lift: the tree splits it, and the split is a command of its own.
- A range that **ends before it starts** is refused, and so is a node
  that is not a track.

### What the port found

- **The tree's extract is the tree's lift plus one shift.** The tree's
  `extract` calls the same `clear_track_range` the lift calls, and
  then **shifts** the clips that were after the range up by the length
  of the range: `shift_track_from(tr, range.end(), -range.duration)`.
  So the two tree commands are one operation and one operation plus a
  shift — and here they are two commands, because a command that
  removes a range and closes a gap does two things.
- **The tree's extract moves the sync-locked tracks as well.** The
  tree's extract closes the gap on the tracks that are **sync-locked**
  with the target, and refuses when a locked track would have to move
  into time that is taken: "this edit would break sync: `A2` is
  sync-locked and has a clip in the way". Here a track is a track, and
  there is no lock to be in.
- **The tree's extract and lift share the removal, and the removal
  splits.** Both tree commands call `clear_track_range`, which splits
  the clips at the edges of the range and removes the clips inside
  it. Here the split is refused rather than done, so the two commands
  here share the *finding* of the clips and not the splitting.

### What it cost

- **Four tests** in the film battery (`tests/extract.rs`): the gap
  closes, a clip that spans an edge is refused, a range that ends
  before it starts is refused, and a node that is not a track is
  refused.
- **One command** (`film.clip.extract`). The film battery now carries
  **13** commands and the MCP server **362**.

### What it does not settle

- **The tree's sync-locked tracks are not here**, and neither is its
  **locked** track, its **links**, its **transitions** nor its
  **caption tracks** — the tree's extract closes the gap on the
  caption tracks that are sync-locked as well.

---

## 274. The modes of the fill

§270 gave the fill **two** modes, and one of them was the tree's
`extend` and not the tree's `series`. The tree has **four**, and they
differ in what they do with a lone number — so the modes here are the
tree's now.

### What was built

- The modes are the tree's: `copy`, `series` — the tree's **default** —
  `extend` and `alternate`. And the tree's table is the table:

  | the source | `copy` | `series` | `extend` | `alternate` |
  |---|---|---|---|---|
  | one number | copied | **copied** | counts up | counts up |
  | two or more numbers | copied | the **trend** | the trend | copied |
  | text with a number | copied | counts on | counts on | copied |
  | text | copied | copied | copied | copied |

- Two or more numbers are continued by the **trend**: the **least
  squares** line through them, so `1, 3` goes on as `5, 7`.
- The source may be a **range**, and the fill goes **lane by lane**:
  each column of the source continues down its own lane, and each row
  of the source continues along its own lane.
- A mode that is **not a mode** is refused, and the refusal names the
  four that are.

### What the port found

- **§270 was wrong about the tree's series, and the tree's own tests
  say so.** The tree's test `ctrl_drag_fill_swaps_copy_and_series` says
  it in a line: "a plain drag copies a lone number" — and the plain
  drag is the tree's *default* mode, which is the series. The tree's
  `extend` is the right-drag **Fill Series** ("a lone number counts
  up, everything else extends as a plain drag"), and the tree's
  `alternate` is the `Ctrl`+drag, which counts a lone number up and
  copies what a plain drag would extend. The port found this by
  reading the tree's `Pattern::detect`, which **copies** a single
  number, and the tree's `alternate`, which turns that copy into a
  count.
- **The tree's trend is a least squares line, not a difference.** The
  tree's `Pattern::detect` fits the **least squares** line through the
  numbers of the source — the trend Excel uses — and continues it. So
  `1, 3` goes on as `5, 7`, and a source of `1, 2, 4` goes on as
  `5.33`, `6.83`, `8.33`: the line through the numbers, and not even
  through the last of them — the tree continues the **fitted** line,
  not the last value. Here the same: the fill continues the line.
- **The tree's source may be a range, and then the fill is a fill per
  lane.** The tree's `fill` takes the **lanes** of the source — its
  columns when the fill goes down, its rows when it goes along — and
  every lane continues its **own** pattern. Here the same: a source of
  `A1:B1` fills two lanes, and each lane counts on by itself.
- **The tree's source with no common pattern continues each cell's own
  series.** The tree's `per_cell` finds, for a source whose cells have
  no pattern in common, the pattern of **each cell alone** and
  continues it: a source of `Product 1` and `On backorder` goes on as
  `Product 2` and `On backorder`, and then `Product 3` and
  `On backorder`. Here the same: in the series and the extend modes a
  copied lane continues each of its cells.

### What it cost

- **Seventeen tests** in the grid battery (`tests/fill.rs`): a copy
  fill copies the value, the series copies a lone number, the extend
  counts a lone number up, the alternate counts a number up and copies
  a text, two numbers are a trend, a series fill counts the number in a
  text on, a text with no number is copied, a range source fills lane
  by lane, the fill goes up, the fill goes right, a target that does
  not hold the source is refused, a source that is not at a corner is
  refused, a source that holds a formula is refused, an empty source
  is refused, a mode that is not a mode is refused, a fill that is too
  large is refused, and a sheet that is not there is refused.
- **No new command**: the command is the same one, with the tree's
  modes. The grid battery still carries **11** commands and the MCP
  server **362**.

### What it does not settle

- **The tree's lists are not here** — the days, the months, the
  quarters — and a text that is a day of the week is copied here, and
  the tree would step through the week. The tree's `days`, `weekdays`,
  `months` and `years` **modes** were not here either, and they are
  now (§277).
- **The tree's `formats` and `values` modes are not here**: the first
  copies the style without the value, the second copies the value
  without the style.
- **A formula is not filled** — and that is no longer true: a formula
  is filled, and its references are shifted (§275). The tree's **flash
  fill**, which reads the pattern out of the text beside the source, is
  still not here.

## 275. The fill of a formula

§274 left the fill of a formula out and said so: the tree copies a
formula and **shifts its references**. That is the tree's
`shift_relative`, and it is here now.

### What was built

- A source that holds a **formula** is filled with the formula, and
  the references of it are shifted for every cell that is written:
  `=B1` in `A1` becomes `=B2` in `A2` and `=B3` in `A3`.
- The **anchoring** of an address is in the model: `$A$1` does not
  move, `A$1` keeps its row, `$A1` keeps its column, and `A1` moves in
  both.
- A reference that would **leave the sheet** is refused, and the
  reference is named.
- A fill that would make a cell **read itself** is refused, and the
  whole fill with it — not a part of it.
- The formula language grew a **printer**: a formula is written out
  again, and the copy a fill writes is the printer's text.

### What the port found

- **The `$` is a part of the address, and the port had thrown it
  away.** v2's `CellRef::parse` skipped the `$` and read `$A$1` as
  `A1` — so here a formula that named an anchored cell and one that
  named a plain cell were the same thing, and the difference the tree's
  `$` makes could not be made at all. Now an address is an address
  **and** its anchoring, and the anchoring is what says whether a
  reference moves. And a command that names a cell names a cell:
  `grid.cell.set` with `A$1` is refused, because `$A$1` is the address
  of a *reference* and not the address of a cell.
- **The tree's shift moves the relative parts and keeps the anchored
  ones — and it moves the references of another sheet too.** The tree's
  `shift_relative` shifts every reference of the expression, whether it
  names this sheet or another: `Sheet2!A1` is a relative reference to
  another sheet and it moves with the copy. Here the same: a fill of
  `=Sheet2!A1` writes `=Sheet2!A2`.
- **The tree's `#REF!` is a value, and a value that says "this
  reference is broken".** The tree's `shift_anchor` refuses an address
  that leaves the sheet, the formula becomes `#REF!`, and the cell
  still holds a formula — a broken one. Here the whole **fill** is
  refused and the reference is named: a refusal that never truncates,
  and a tree that would rather break a formula than stop.
- **A fill can make a cell read itself, and the tree does not look.**
  The tree's fill copies a formula and moves its references, and
  nothing in the fill asks whether the copy reads the cell it is
  written in: `=B1` in `A1`, filled into `A2`, reads `B2` — and if
  `B2` reads `A2` the two cells read each other. Here the copy is
  checked against the **workbook** before it is written and the fill is
  refused, with the cell named — the same rule the formula command has,
  where the cycle is found at the point the formula is set and not
  later in the middle of a recalculation.
- **A formula cell has no value to offer a pattern.** The tree's fill
  detects its pattern from the cells' **values**, and a formula cell's
  value in the tree is the tree's **cached** one. Here a formula cell's
  value is derived and never written down, so a lane's pattern is made
  from the cells that hold values, and a cell that holds a formula is
  given its formula instead.

### What it cost

- **Twelve tests** in the grid battery (`tests/formula_fill.rs`): a
  fill of a formula shifts its references, a copy of a formula holds a
  formula and not a value, an anchored reference does not move, a range
  in a formula shifts as a whole, a reference to another sheet keeps
  its sheet, the mode does not matter for a formula, a fill that makes
  a cell read itself is refused, a reference that would leave the sheet
  is refused, the value of a copied formula is derived, a fill of a
  range of formulas shifts every one of them, a fill of a formula
  copies the format, and a text is not a formula.
- **One test taken away**: `tests/fill.rs` had a test that said a
  source that holds a formula is refused, and it is not true any more.
- **The formula model changed**: `Expr::Cell` and `Expr::Range` carry
  an **anchor** now, the language has a printer, and the printer's text
  is what a fill writes.
- **No new command**: the grid battery still carries **11** commands and
  the MCP server **362**.

### What it does not settle

- **The tree's flash fill** is not here: it reads the pattern out of
  the text beside the source, and that is a thing of its own.
- **The tree's `formats` and `values` modes** are still not here, and
  neither are its **lists** — the days, the months, the quarters — nor
  its **dates**.
- **The tree's whole-column and whole-row references** — `A:A`, `1:1`
  — are refused by the parser here, and so are the tree's **defined
  names**, its **arrays** and its `INDIRECT`: a call this tree does not
  have is kept and its value is an error.

## 276. The date of a cell

§271 said the tree's dates were not here, and named them among the
things that are missing. They are here now.

### What was built

- A **date format** is a format, and there are **nine** of them:
  `yyyy-mm-dd`, `dd/mm/yyyy`, `m/d/yyyy`, `d-mmm-yyyy`, `mmm`, `mmmm`,
  `ddd`, `dddd` and `yyyy`. A code that is not one of them is refused,
  and the refusal names them.
- A **date is a number with a date format on it** — the number of days
  since the 30th of December 1899 — and the command `grid.cell.date`
  puts both of them in a cell.
- A date may be written the **tree's** way: `2026-10-26`,
  `10/26/2026` or `4-Mar-2026`, and the format the date is written with
  is the **shape it was written in**.
- A date that **did not happen** is refused: the 30th of February is
  not a date.

### What the port found

- **A date in a spreadsheet is not a value: it is a number and a
  format.** The tree's cell holds a `Value` and a `Style`, and the
  style carries the number format; a date is the number of days and the
  format that says so. Here the format is the cell's own field (§271)
  and the date is the same shape: the number, and the format. That is
  why a date and a number are not two kinds of thing here — they are
  one kind of thing with a format on it.
- **The tree's calendar has a day that never happened.** The 1900
  system counts from the 30th of December 1899 and — like Lotus 1-2-3
  and Excel, and for a mistake that is a hundred and twenty-five years
  old — it treats 1900 as a **leap year**: the serial 60 is the
  fictitious 29th of February 1900. So the tree's first two months of
  1900 have sixty serials and fifty-nine days, and the tree's serial 1
  is the 1st of January and its serial 61 is the 1st of March. Here the
  same: `1900-02-29` is a date this tree accepts, and it is the
  serial 60.
- **The tree's weekdays follow the fictitious calendar, and after the
  phantom day they are right.** The tree's `datetime_from_serial`
  computes the weekday from the serial alone, so the serial 1 is a
  Sunday — a day out from the truth — and the serial 61 is the Monday
  the 1st of March 1900 really was. Here the same: the weekday comes
  from the serial and not from the date, and the 26th of October 2026
  is a Monday.
- **The tree's date formats are a language, and here they are a
  list.** The tree's `numfmt` parses a code with `y` and `m` and `d`
  and `h` and `s` in it, with sections, with colours and with fills;
  `mmm` is the month in three letters and `dddd` is the day of the week
  in full, and the tree renders them. Here a date format is one of
  nine shapes and a code that is not one of them is refused.
- **The tree turns what you type into a date, and it reads the month
  first.** The tree's `parse_date` reads `2026-10-26` as the year
  first, `10/26/2026` as the **month** first — so `26/10/2026` is not
  a date to it — and `4-Mar-2026` with the month by its name; and it
  reads a year of two digits as this century below thirty and as the
  last one above thirty. Here the same, and the shape a date was
  written in is the format it is written with.
- **The tree rolls a date over and this tree refuses it.** The tree's
  `serial_from_ymd` is `DATE()`: the month 13 is the January of the
  next year and the day 0 is the last day of the month before. A cell
  that holds a date holds a date that **was**, so here the 30th of
  February is refused.

### What it cost

- **Thirteen tests** in the grid battery (`tests/dates.rs`): a date is a
  number with a format on it, the 1st of January 1900 is the serial 1,
  the tree has a day that never happened, a date that did not happen is
  refused, a date that is not written as one is refused, a format that
  is not a date format is refused, a date may be written in any of its
  shapes, a date may be written the tree's way, the tree reads the
  month first, a year of two digits is the tree's, a number with a date
  format is a date, a format this tree does not have is refused, and a
  number that is not a date is a number.
- **One new command**: `grid.cell.date`. The grid battery now carries
  **12** commands and the MCP server **363**.
- **The format list grew**: seven number formats and nine date
  formats, and the refusal that names them names sixteen.

### What it does not settle

- **The tree's 1904 date system is not here**: it is the system of the
  old Macintosh Excel, and a workbook that says which system it uses is
  a workbook that was made somewhere else.
- **The tree's times are not here**: there is no `hh:mm`, no seconds,
  no a.m. and p.m., and a date with a time in it is a number here.
- **A fill of dates is not here yet**: the tree's `days`, `weekdays`,
  `months` and `years` modes step through the calendar, and the
  arithmetic they need is here but the modes are not (§274).
- **The tree's date and time parsing is not here**: the tree reads a
  time of day as well as a date, and a date with a time in it, and
  here a date is a date.

## 277. The fill of dates

§276 gave the grid a **date**, and §274 said the tree's four date fills
were not here. They are here now.

### What was built

- The fill has **four more modes**, and they are the tree's: `days`,
  `weekdays`, `months` and `years`.
- A fill of **days** steps by days, and the step is the difference
  between the first two dates: two dates a week apart are a fill of
  every seventh day.
- A fill of **weekdays** skips the Saturday and the Sunday.
- A fill of **months** keeps the day of the month and **clamps** it to
  the month it lands in — the 31st of January, the 28th of February,
  the 31st of March — and it does not drift.
- A fill of **years** steps by years, and a year is twelve months.
- A source that is **not dates** extends like a plain drag, as the
  tree has it.

### What the port found

- **The tree's date modes are not the plain series with a different
  step: they are a pattern of their own.** The tree's `Pattern::dates`
  is asked for the date modes first, and it needs the source to be
  numbers under a **date format**; when it cannot have that it falls
  back to the plain detection, and a lone number is copied. So the
  modes are a fill of dates and nothing else, and a fill of a number
  in the `days` mode is a plain drag. Here the same: the date modes
  ask the source whether it holds dates, and a source that does not
  extends like a plain drag.
- **The tree's `days` step is the difference between the first two
  dates, and not one day.** Two dates a week apart are a fill of every
  seventh day — and two dates on the same day are a fill of one day,
  because the tree's rule is that a step of zero is one. Here the same.
- **The tree's `weekdays` step counts the weekdays between the first
  two dates, and skips the weekend on the way.** The tree's
  `add_weekdays` moves to the next weekday first — so a fill that
  starts on a Saturday starts on the Monday — and then counts five
  weekdays to the week. Here the same.
- **The tree's months are clamped and they do not drift, and that is
  the tree's own comment on it.** The tree's `add_months_clamped`
  keeps the day of the month and clamps it to the length of the month
  it lands in, and it always measures from the **original** date, so
  a series is the 31st of January, the 28th of February, the 31st of
  March and the 30th of April — and not the 28th of February, the 28th
  of March and the 28th of April, which is what a fill that added a
  month to the cell before it would give. Here the same.
- **The tree's plain series knows about dates too, and the port had
  forgotten.** The tree's `Pattern::detect` looks at the format of the
  source: a **single date** counts a day on — where a single *number*
  is copied — and two dates of the **same day of the month** go on by
  months and not by a line through them. Here the same, and that is a
  rule the tree had and this tree did not: the port had a fill of
  numbers and a fill of texts, and a date was a number with a format
  on it and the fill never looked at the format.
- **The tree's year is twelve months, and its month step is a count of
  months and not a count of days.** Two dates two years apart are a
  fill of every second year, and the tree's rule is that a count of
  zero is one — so two dates in the same month are a fill of a year.
  Here the same.

### What it cost

- **Eleven tests** in the grid battery (`tests/date_fill.rs`): a fill
  of days steps by days, two dates a week apart step by a week, a fill
  of weekdays skips the weekend, a fill of months clamps the day and
  does not drift, a fill of years steps by years, two dates years apart
  step by years, a source that is not dates extends like a plain drag,
  a single date in a plain series counts a day on, two dates of the
  same day go on by months, two dates of different days go on by a
  line, and a mode that is not a mode is refused.
- **One test changed**: the refusal of a mode that is not a mode now
  names eight modes and not four.
- **No new command**: the command is the same one, with four more
  modes. The grid battery still carries **12** commands and the MCP
  server **363**.

### What it does not settle

- **The tree's lists are not here**: the days, the months, the
  quarters and the ordinals, and a text that is a day of the week is
  copied here and the tree would step through the week.
- **The tree's flash fill is not here**: it reads the pattern out of
  the text beside the source.
- **The tree's `formats` and `values` modes are not here**: the first
  copies the style without the value, the second copies the value
  without the style.

## 278. The `TEXT` of a formula

§271 said the tree's `TEXT` was not here, and said why: "a formula in
this tree has no format code in it". §276 gave the tree its date
formats, so the reason is gone — and the reason was wrong in any case.

### What was built

- The formula language has the tree's **`TEXT`**: `TEXT(value, code)`
  writes a value with a **format code** and the answer is a text.
- The code is **evaluated**, so it may be a cell that holds a code.
- A code that this tree does not have is an **error**, and the error
  names the code.

### What the port found

- **The reason §271 gave was wrong, and the port had carried it for
  eleven arcs.** §271 said a formula in this tree has no format code
  in it — and the truth is that a formula may name **any** code, and
  what it does with it is the question. The codes are here (§271 for
  the numbers and §276 for the dates), so `TEXT` is a function of the
  format module and not a new thing.
- **The tree's `TEXT` takes the second argument as a value, not as a
  code.** The tree's `TEXT` evaluates both of its arguments and then
  asks the second one for its text: so `TEXT(A1, B1)` writes `A1` with
  whatever code is **in** `B1`. Here the same: the code is a value, and
  a value may be in a cell.
- **The tree's `TEXT` is an error when the value is an error, and it
  does not go on to write anything.** Here the same: the error of the
  value is the error of the `TEXT`, and a `TEXT` with one argument or
  with three is `#VALUE!`.
- **The number formats of this tree are a grammar, and §271 said they
  were a list.** §271 named seven number formats and said "a code
  that is not one of them is refused" — and the seven are the ones the
  tree's own tests use, while the **code** takes any number of
  decimals and any arrangement of `0` and `#`. So `0.0000` and `#0`
  have been formats this tree has had since §271, and the claim in
  the README and in the crate's own words was false. It is corrected
  here: the number formats are a small **grammar** — the thousands, a
  whole part of placeholders with at least one `0` in it, up to
  fifteen decimals and a percent sign — and the date formats are a
  **list** of nine, because the tree's date formats are a language
  and this one has nine shapes of it.
- **A format that is refused by `grid.cell.format` is not refused by
  `TEXT`, and that is a difference worth naming.** The command refuses
  a code it does not have, because a command that sets a format is a
  user writing a format; a formula that writes a value with a code it
  does not have is an **error in a cell**, and the tree's own way of
  saying that is the cell's error.

### What it cost

- **Nine tests** in the grid battery (`tests/text_function.rs`): a
  `TEXT` writes a number with a format code, a `TEXT` of a percentage
  multiplies it, a `TEXT` of a date writes the date, the code may be
  in a cell, a code this tree does not have is an error, a `TEXT` with
  the wrong arguments is an error, a `TEXT` of a text is the text, a
  `TEXT` of a general format is the value, and a code with four
  decimals is a code this tree has.
- **Two claims corrected**: §271's list of seven number formats and
  the reason it gave for the tree's `TEXT` not being here.
- **No new command**: a function is not a command, and the formula
  command is the same one. The grid battery still carries **12**
  commands and the MCP server **363**.

### What it does not settle

- **The tree's `TEXT` takes the tree's whole format language** and
  this one takes the grammar and the nine dates of this tree: a code
  with a section, a colour or a fill is an error here and a text in
  the tree.
- **The tree's other functions are not here**: its `IF`, its `INDEX`,
  its `XLOOKUP`, its `LET` and its `LAMBDA`, and the tree's
  **volatile** functions.
