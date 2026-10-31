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

**1032 tests.** The MCP server links **all fifteen batteries** in, so the
workspace's count is the same with or without the studio's features
(`craftkit-studio/vector`, `/sound`, `/pdf`, `/film`, `/fx`, `/art`,
`/grid`, `/cad`, `/prose`, `/layout`, `/numeric`, `/text`, `/image`,
`/clap`, `/vst3`) — and the studio
on its own, with no features, carries **238** of them, and **252** with
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
cargo test          # 1032 tests
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
| a **real document model**: layers, text, images, an appearance of its own | **partly.** Layers are nodes and edges (§44); text is an element in two batteries (§33, §44), a paragraph's own text and runs in a third (§49), and a paragraph **broken into lines** in a fourth (§58) — with the widths coming from the caller and the **letters** from the kit's own typeface (§128, §129); **images are not ported**, and an appearance is still a plugin's payload | §44, §49, §58 |
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
| **hyphenation**: where a word may be broken | **not built.** The breaker knows a **hyphenation point** — a flagged penalty — and the limit on how many may follow each other, but **finding** them is a dictionary's business, and no dictionary is here | §58 |

### The list of things that are honestly not there: empty

It is empty, and that is not the end of the work — it is the end of
*this* list. The table above still has rows that are not built (the
toolbar, by design; the plugin formats, which are a shim over a door
that is built; images, which are a port nobody has made yet), and
those are **not** on this list because they are not gaps in the kit's
design: they are things this tree has not been asked for.

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
- **Hyphenation is a dictionary's business.** The breaker knows a
  **hyphenation point** — a flagged penalty — and it enforces the limit
  on how many flagged breaks may follow each other, as a **hard
  constraint**: a break that would exceed it is not taken, in both
  breakers. But *finding* the points is the dictionary's work, and the
  family's dictionary carries forty-eight languages — one of them under
  a licence that is not this tree's. So the point is in the model and
  the dictionary is not, and the two are separated in the same way the
  font is.

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
  (§127), and now the samples are in the room. — **Answered in §134
  and §135**: the engine is the mixer, and the document is rendered
  into a file. What is still missing is a **device** and a
  **playhead**.
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
  refuses such a source (§135) rather than resampling it.
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
  along it.
- **The mixer's graph is not rendered.** The buses, the sends, the
  inserts and the automation lanes (§127) are in the plan and not in
  the render: the render sums the clips.
- **There is no resampler and no time stretch.** A source at another
  rate is refused, and a clip that should be slower than its source is
  not made.
