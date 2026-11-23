# The a2ui compat layer: design and implementation plan

**Meta-programming first.** The compiler writes the declaration, the reader,
the schema, the catalogue entry and the registration; the hand-written part
is the painting, because the painting seam is craftkit's own rule.

This is the implementation plan for the a2ui-compatible surface — **O11** of
`survey-a2ui.md` — and it **supersedes O11's scope**. Where they differ,
this plan is the one that is true:

| | O11 as written | this plan |
|---|---|---|
| crates | **one**, at L4 | **three** (L3, L4, L5) plus **three changed** (`craftkit-sdk`, `craftkit-derive`, `craftkit-plugin-vector`) |
| the SDK | *"Nothing in the SDK changes"* | the SDK **changes**: a `CatalogEntry` contribution, a catalogue projection, and the schema mapping moved in from `craftkit-mcp` |
| the macro | "a new derive macro", unnamed | **one**: `#[derive(A2uiComponent)]`, and `components! { … }` beside it |
| components | **six** | **eleven** — and **eight** of them need work the plan names |

So the **O11 body** in §4 of `survey-a2ui.md` was wrong on three counts —
the **crate count** (*"one, at L4"*), the **component count** (*"six kinds"*),
and **"Nothing in the SDK changes"** — and this plan is the one that is true.
What it adds to what O11 had is the **crate** split, the **five** commands,
the **SDK** changes and the **named** macro.

Its §6.2 cost row and its action semantics have been corrected to match
too; §5 says why an action is two things rather than one, and §5.1 says why
a function call is resolved local-first.

---

## 0. What is being built

**craftkit takes a2ui surfaces and renders them, with its own kinds, through
its own seam, and an a2ui-speaking agent drives it with craftkit commands.**

In a2ui's terms craftkit is the **renderer**: the agent sends `createSurface`
and `updateComponents`, and craftkit turns them into nodes that paint like
every other node. In craftkit's terms it is **one more battery**: a crate
that contributes kinds and commands, beside the others.

The constraint that shapes everything:

> The compat layer **adds**. It does not replace a kind, touch the rasteriser,
> touch the camera, touch the font metrics, touch the text layout, or add a
> second way to run a command.

---

## 1. The shape

```text
   an a2ui-speaking agent
            │
            │  createSurface · updateComponents · updateDataModel
            │  deleteSurface · callRendererFunction · agentFunctionResponse
            ▼
   ┌──────────────────────────────────────────────┐
   │  apps/craftkit-a2ui-server  (L5, an app)       │  the door: speaks a2ui
   ├──────────────────────────────────────────────┤
   │  crates/craftkit-plugin-a2ui  (L4, a plugin) │  the components: kinds,
   │                                              │  painters, catalogue rows
   ├──────────────────────────────────────────────┤
   │  crates/craftkit-plugin-vector (L4, a plugin) │  its SVG path-data reader,
   │                                              │  which moves down to the SDK
   ├──────────────────────────────────────────────┤
   │  crates/craftkit-a2ui      (L3)              │  the protocol, the data
   │                                              │  model, the bindings, the
   │                                              │  box layout, the a2ui
   │                                              │  catalogue projection
   ├──────────────────────────────────────────────┤
   │  crates/craftkit-sdk       (L2)              │  CatalogEntry, the schema
   │                                              │  view, the vocabulary view,
   │                                              │  the SVG path-data reader,
   │                                              │  and the Doc → JSON Schema
   │                                              │  mapping every door shares
   ├──────────────────────────────────────────────┤
   │  crates/craftkit-derive    (L0)              │  #[derive(A2uiComponent)]
   │                                              │  components! { … }
   └──────────────────────────────────────────────┘
            │
            ▼  ordinary Canvas seam
      RecordingCanvas · Raster · the window
```

**One direction only.** a2ui in, craftkit out.

**And the guarantee, stated precisely** — because the absolute form of it is
false: the plan's components are craftkit **node kinds** named `a2ui.text`,
`a2ui.column` and so on, in a layer named `a2ui`, so craftkit's own registry,
manifest and projections **will** carry the name. What is true is:

> **The core does not depend on a2ui.** The **SDK** knows nothing about it,
> the **derive** gains one a2ui-specific macro that emits no a2ui *type* — it
> emits a **path** into the a2ui crate, and a check, and nothing else — and
> a binary without the plugin has no `a2ui.*` kinds at all.

---

## 2. Meta-programming first: what the compiler writes

### 2.1 The two spellings, like commands

The tree has two spellings for a command — `#[derive(Command)]` for one, and
the `commands! { … }` table for a battery. A component gets the same pair:

| | one | many |
|---|---|---|
| command | `#[derive(Command)]` | `commands! { … }` |
| **component** | `#[derive(A2uiComponent)]` | `components! { … }` |

The compat crate uses the table, because a catalogue *is* a table.

### 2.2 The props are craftkit params — and the **published** schema is a2ui's

The decisive move: **an a2ui component's props are craftkit parameters.** So
the prop's *type* says what it is, the doc comment says what it is for, and
the machinery already exists:

- `ParamKind` — `DOC`, `NAME`, `UNIT`, `REQUIRED` and `read`
  (`crates/craftkit-sdk/src/params.rs:138-178`)
- `Doc` — the kinds: `Text`, `Number`, `Bool`, `NodeRef`, `NodeKind`, `Json`
  (`:34-50`)
- `ParamDoc::required` / `optional` / `either`
  (`crates/craftkit-sdk/src/error.rs:210-244`)
- `Option<T>` — optional (`crates/craftkit-sdk/src/params.rs:336`);
  `Either<A, B>` with `#[param(or = "…")]` (`:368`)

A prop is a `Text`, a `Number`, an `Option<Number>` — nothing new.

**But there are two schemas, and they are not the same.** This is the
correction this plan's first draft got wrong, and it matters because the
catalogue is what an a2ui agent generates against.

| | the **published** schema | the **craftkit** props |
|---|---|---|
| what it is | a2ui's shape: a2ui's prop **names**, a2ui's prop **types** | the resolved values, as craftkit types |
| who reads it | the **agent**, which sends `text`, `variant`, `children`, `action`… | the **component**, after the layer has resolved the bindings |
| where it comes from | the row's declared props | the same row, after resolution |

**a2ui's props are not plain values.** a2ui's `text` is a **`DynamicString`**
— a literal, **or** a `{"@path": "/title"}` binding into the data model, **or**
a function call (`common_types.json`'s `$defs/DynamicString`). A schema that
said "string" would refuse every bound prop.

**And so the two schemas differ in their *type*, not in their *names*.** Both
sides use **a2ui's** prop names — `text`, `variant`, `children`, `action` —
because those are the names the agent sends and the names the component is
written against. What differs is the shape of a value:

| the prop | the **published** schema says | the **resolved** prop is |
|---|---|---|
| `text` | a `DynamicString` — a string, **or** a binding, **or** a call | a `Text` — a string, and nothing else |

**And the catalogue carries its own `$defs`.** The published schema is a2ui's
*shape*, and a shape that referred to another file would be a file with a
**dangling reference** unless the second file were served beside it. So the
projection **inlines** the definitions into the catalogue's own `$defs`, and
every reference is **local**: `"$ref": "#/$defs/DynamicString"`. **One file,
self-contained, nothing to fetch.**

**And the definitions are *data*, not something the compiler can invent.** A
prop table that says "`text` is a `DynamicString`" yields the **name**
`DynamicString` and never its **shape** — and the shape is a2ui's:
`DynamicString` is a `oneOf` of a string, a `DataBinding` and a `FunctionCall`;
`FunctionCall` is an `allOf` of `FunctionCommon` and a `oneOf` of two more.

**And they are *re-derived*, not copied.** The tree's clean-room rule says
that *"behaviour is designed from public specifications, general conventions
and user feedback"* and that *"nothing is copied from another project's
source"* (`docs/clean-room.md:9`). a2ui's JSON Schemas are a **public
specification** — the machine-readable form of the protocol — so **reading
them is designing from a specification**, and a2ui is **permissive**, not
copyleft, so the rule against reading a copyleft project does not touch it.
But **no a2ui file enters this tree**: the layer's definitions are
**craftkit's own**, written from the **meaning** of the specification, and the
compiler emits them from a table in `craftkit-a2ui` (`src/defs.rs`) that
states each definition in **our own words**. So:

- the closure is **`common_types.json`'s** definitions **plus `Weight`** —
  and `Weight` is a *number*, so it is written here rather than carried;
- **`anyComponent` and `anyFunction` are written by the projection** (Arc F),
  not carried at all (they are the catalogue's own, and carrying a2ui's would
  dangle — see below);
- and **nothing is copied**, so **`ATTRIBUTION.md` needs no row for it** —
  which is the point, because the house asks that file for *"every piece of
  source code adapted from another project"*, and nothing here is adapted.

**And there is no rewriting step, because there is no transcription.** The
re-derived definitions are **authored with local references from the start** —
`"#/$defs/DynamicString"`, never `"common_types.json#/$defs/DynamicString"` —
so the published catalogue names its definitions locally, and the catalogue
is **one self-contained file**. Nothing of a2ui's file spelling enters the
tree, so there is nothing to rewrite and nothing to attribute.

**And the same is true of the *props*, and this is the plan's deepest
correction.** a2ui's prop shapes are **not** expressible in the row's
vocabulary: `Text.variant` is an `enum` of `["caption", "body"]`, `Icon.name`
is a `oneOf` of an **object with `svgPath`** and a binding, and `Image.fit`
is an enum. **A row cannot say an enum.** So the plan says it plainly:

| | the single source of truth |
|---|---|
| **what the props are** — their names, their craftkit types, which a2ui definition each uses | **the row** |
| **the shapes** — the enums, the `oneOf`s, the descriptions | the **re-derived** shapes, in `craftkit-a2ui/src/schema.rs` |

**And there are three artefacts and three owners, and three checks.** The
plan had two of them fighting for the same job; here is the settled shape:

| the artefact | where | what it owns |
|---|---|---|
| the **table** — each component's property names, as byte arrays | `craftkit-a2ui/src/defs.rs` | the **names** |
| the **schema text** — a2ui's shape for each component | `craftkit-a2ui/src/schema.rs` | the **shapes** |
| the **row** — the props, in craftkit types | the plugin | the **resolved** side |

And the checks, one per pair, so that **no direction is uncovered**:

| the check | when | what it catches |
|---|---|---|
| the **row** against the **table** | **compile time** — a `const` assertion | a prop the row declares and the table does not hold, **and** a prop the table holds and the row does not declare |
| the **table** against the **schema text** | **a test** (Arc F) | a prop the table holds and the schema text does not |
| the **row** against the **published** schema | **a test** (Arc **G**) | a prop the row declares and the published schema does not carry. **The comparison skips the exempt entries** — the same rule the Arc F check uses |

**And the published schema is *written by the compiler*** — from the
re-derived shapes, the row's names, the **local** `$ref`s, the stamped
discriminator and the declared narrowings — and the row is what says **which**
component it is.

**And the check is a `const`, not an expansion-time lookup — because the
derive is L0 and the shapes are L3.** A macro cannot read another crate's
data at expansion time, and it may not: `craftkit-derive` (L0) may not
depend on `craftkit-a2ui` (L3). So the shapes live in `craftkit-a2ui` as
**two artefacts**, and the split matters:

| the artefact | what it is | what it is for |
|---|---|---|
| `src/defs.rs` — a **`pub const`** table | each component's **property names**, as **byte arrays** (`b"text"`), and the **exempt** set (`component`, `@checks`) | the **check**: the derive emits a **`const _: () = assert!(…)`** against it |
| `src/schema.rs` — the **schema text** | the JSON Schema, as `&'static str` | the **published** schema, at runtime |

**And the table owns the property *names*, and the schema text owns the
property *shapes*** — two separate artefacts, two owners, and the published
schema is **generated from both**. So the table↔schema-text check is a
**real** one, not a tautology. And the check runs over the **table**, never
over the schema text: a `const` cannot parse JSON, and the **exemptions are
encoded in the table** rather than discovered by walking an `allOf`.

**And the comparison is over bytes, because `&str` equality is not
const-evaluable.** `PartialEq` for `str` is not stable in a `const`
context, so the table holds `&'static [u8]` and the check is a **`const
fn`** that walks the bytes:

```rust
/// Whether the table and the names hold the same **set**.
///
/// Order-insensitive, and byte-wise: a slice's `PartialEq` is **not**
/// const-evaluable, and neither is `str`'s.
pub const fn matches(table: &[&[u8]], names: &[&[u8]]) -> bool { … }
```

**And one call covers both directions**, because it is a **set equality** and
not a membership: every name in `names` is in `table`, **and** every name in
`table` is in `names`. The derive emits the **row's** names as a byte-array
literal and asserts `matches(TABLE, NAMES)` — so a prop the row has and the
table lacks, **and** a prop the table has and the row lacks, are both compile
errors.

**And the set is order-insensitive on purpose.** The published schema's
property order is the **JSON** order, not the row's — a JSON object's
members are not ordered by anything craftkit controls — so an order-sensitive
check would fail spuriously. The table is a **set of names**, and so is the
row's.

**And the exempt set is a *separate* constant**, because a name the layer
does not publish is not in the table: `src/defs.rs` holds the names, and
`src/defs.rs` holds the **exempt** names beside them — `component`, which
the compiler stamps, and `@checks`, which the layer removes. The **Arc F**
check — the schema text's property set against the table's — **skips the
exempt entries**, because the schema text is a2ui's and still carries them.

**And the derive emits the assertion**, which runs in the **plugin's**
compilation unit, where L4 → L3 → L2 is allowed — so a mismatch is a
**compile error**, and it is a compile error in the crate that has the row,
which is where the mistake was made.

**And two things are exempt from that check, because the layer writes them
rather than declares them.** The **`component` discriminator** is stamped by
the compiler — every a2ui component's schema carries
`"component": { "const": "<Name>" }` — so the row does not declare it. And
**`@checks`** — which a2ui's `Checkable` gives to `Button`, `CheckBox` and
`TextField` — is **removed**, as a **declared narrowing** beside the `Icon`
enum branch and the `ChildList` template branch, because the layer evaluates
no check rule and a rule it cannot run must not be published.

**And that is the only treatment.** The three checkable components stay
**supported** — the layer drops the `@checks` **property** from their
published schema and says why, and it does **not** refuse a component for
carrying one. A property the layer does not publish and a component the
layer refuses are two different things, and only the first is true here.

**And an `allOf` schema is checked part by part.** a2ui's `Button` is an
`allOf` of `Checkable` and its own object, so the check walks the parts: the
**own** object's properties are the row's, and `Checkable`'s is the exempted
`@checks`.

**And the narrowings are edits to the re-derived shapes**, so they are
visible: the `Icon` enum branch is gone because the shape says so.

**And there are five of them, and they are all the same rule** — *a branch
the layer cannot execute is a branch it must not publish*:

| the branch | why the layer cannot execute it |
|---|---|
| the `Icon` enum of **59 built-in names** | craftkit has **no icon vocabulary**, and drawing 59 glyphs is a large hand-written body |
| the `ChildList` **template** branch | a template is rendered per item with the reserved **`@index`**, and the layer refuses every reserved function |
| the `FunctionCall`'s **`IndexSystemFunction`** branch | that is the reserved **`@index`**, refused for the same reason |
| **`Checkable`'s `@checks`** | the layer evaluates no check rule, and a rule it cannot run must not be published |
| the **`TextField`'s `value`** — the literal and the call branches of its `DynamicString` | a literal has **nowhere to live** — no path to write it to — and a call's **answer** has nowhere to live either, so only the **`DataBinding`** branch is published (§5.2) |

**And the narrowing is a *branch*, not the whole definition**: `Checkable`
stays (it is the `allOf` the component schemas are built on), `ChildList`
stays (its static branch is usable), and `FunctionCall` stays (a call is a
call).

**And the set is the *transitive closure*, not a short list.** `DynamicString`
reaches `DataBinding` and `FunctionCall`; `FunctionCall` reaches
the catalogue's own `anyFunction` — its `@index` branch is **narrowed out**;
`Child` reaches
`ComponentId`; `Action` reaches `DynamicValue`; `Row`'s `children` reaches
`ChildList`. So the closure is
**thirteen** definitions — `DynamicString`, `DynamicBoolean`, `DataBinding`,
`FunctionCall`, `FunctionCommon`, `Child`, `ChildList`, `ComponentId`,
`Checkable`, `Action`, `DynamicValue`, `Weight`, `anyFunction` — **plus** the
catalogue's own **`anyComponent`**, which `catalog_definition.json` requires.
So the published catalogue holds **fourteen** entries in its `$defs`.
(`DynamicStringList` is **not** in the closure: no component the layer
supports uses it. And **`CheckRule` is not either** — a2ui refers to it from
exactly one place, `Checkable`'s `@checks` array, and the layer **removes**
`@checks`, so nothing reaches `CheckRule` any more.

**And neither are `IndexSystemFunction` and `DynamicNumber`.** The
`IndexSystemFunction` — the reserved **`@index`** — is reached from
`FunctionCall`, and `DynamicNumber` only from it. The layer **refuses every
reserved function** (§5.1), so the published `FunctionCall` **narrows the
`IndexSystemFunction` branch out**, exactly as the `Icon` enum branch and the
`ChildList` template branch are narrowed out. **The closure follows the
schemas, and the schemas changed** — three times now, and each time the
count fell by what the narrowing removed. A short list would leave a dangling
reference in exactly the schemas that matter most: `Button`, `Row` and
`CheckBox`.

**And `anyComponent` and `anyFunction` are the catalogue's own**, and they
are **Arc F's** — the *projection* assembles them, because the **`oneOf`**
spans the whole catalogue and no single component's derive can know it. (The
derive emits **one** component's entry, with its own `#/components/<Name>`
reference; the projection collects the entries and writes the **`oneOf`**
over them.) a **`oneOf`** over the catalogue's components, each by its
own `#/components/<Name>` reference, and a **`oneOf`** over the catalogue's
own `functions` — which is what a2ui's own catalogue does
(`catalogs/basic/v1/catalog.json`'s `$defs/anyComponent` and
`$defs/anyFunction`). And it is how the catalogue says "**any component of
this catalogue**" and "**any function of this catalogue**".

**And they cannot be carried, because they would dangle.** a2ui's own
`anyFunction` is a `oneOf` over its **fourteen** functions
(`#/functions/required`, `#/functions/regex`, …), and the layer's catalogue
declares **five** — so carrying a2ui's `anyFunction` would leave **nine**
references pointing at functions that are not there. That is exactly the
dangling reference the whole inlining was to avoid, so the compiler writes
this one, and it writes it over the layer's own functions.

**And the rule is a rule, not a list.** The set is **the transitive closure
of every `$ref` reachable from the component schemas** — so it is *derived*
from the schemas and cannot fall behind them, and the count above is what
that closure is today, not a promise about tomorrow.

An agent that binds `text` to `/title` sends a **path**; the layer resolves
it; the component reads a **string**. Two schemas, one prop, one name.

So the row declares each prop with its **a2ui type**, and the compiler writes
both sides from one row:

| the row says | the published schema carries | the resolved craftkit prop |
|---|---|---|
| `#[a2ui_prop(DynamicString)]` | `"$ref": "#/$defs/DynamicString"` | `Text` |
| `#[a2ui_prop(DynamicBoolean)]` | `"$ref": "#/$defs/DynamicBoolean"` | `Bool` |
| `#[a2ui_prop(Weight)]` | `"$ref": "#/$defs/Weight"` | `Option<Number>` |
| `#[a2ui_prop(Child)]` | `"$ref": "#/$defs/Child"` | `Text` |
| `#[a2ui_prop(ChildList)]` | `"$ref": "#/$defs/ChildList"` | `Json` |
| `#[a2ui_prop(Action)]` | `"$ref": "#/$defs/Action"` | `Json` |
| **no** `#[a2ui_prop]` | the layer's **inline** shape for that prop — the enum, the `oneOf`, or the string | the craftkit type of the field |

**And the row's vocabulary is a2ui's own definition names, literally.** There
is no shorthand table to keep in step: the attribute names the **definition**,
and the layer's `schema.rs` holds a shape for each name — either a `$defs`
reference or an **inline** shape. So `text` is a `DynamicString`, `weight` is
a `Weight`, `Button`'s `child` is a `Child`, `Row`'s `children` is a
`ChildList`, and `Button`'s `action` is an `Action`.

**And `Icon`'s `name` is a plain prop** — a2ui gives it an **inline** `oneOf`
(the 59-name enum, the object with `svgPath`, and a binding), not a
`$defs` entry, so there is no name to cite. The layer owns that inline
shape, and the **narrowed** one is published — the 59-name branch gone.

**And a prop with no `#[a2ui_prop]` is a plain one**, and the layer's
**inline** shape owns it: `variant` is the enum `["caption", "body"]`, `axis`
is `"horizontal"` or `"vertical"`, `fit` is a2ui's own enum. The row says the
prop's **craftkit** type; the layer says its **published** shape.

**The resolution happens before the read** (§5), so the component's props are
plain craftkit params and the generated reader is used unchanged.

### 2.3 What a row emits

A row of `components! { … }`, mirroring `commands!`'s grammar — including its
**body slot**, because the painter stays hand-written:

```rust
components!(
    table = "craftkit_a2ui::defs",
    shapes = "craftkit_a2ui::schema"
) {
    /// A line of text.
    ///
    /// The doc comment is the component's one line about itself: it is what
    /// the catalogue shows and what the schema says.
    Text {
        kind: "a2ui.text",
        props: {
            /// The text to show.
            #[a2ui_prop(DynamicString)]
            text: Text,
            /// A hint for the base text size.
            variant: Option<Text>,
            /// How much of the row this takes.
            #[a2ui_prop(Weight)]
            weight: Option<Number>,
        },
        paint: {
            let Some(props) = TextComponent::of(node) else { return };
            let Some(rect) = node.rect else { return };
            let colour = document
                .appearance_of(node)
                .map(|appearance| appearance.fill.colour())
                .unwrap_or(Colour::BLACK);
            let at = Point::new(rect.x, rect.y);
            // The author's own helper: a2ui's `variant` is a name, and
            // what size a name means is this program's business.
            let size = size_of_variant(props.variant.as_ref().map(|variant| variant.0.as_str()));
            canvas.text(at, &props.text.0, colour, size);
        },
        hit: {
            node.rect.map(|rect| rect.contains(point)).unwrap_or(false)
        },
    }
}
```

**The painter is ordinary craftkit code.** Every name in it is the tree's
own: `node.rect` (`crates/craftkit-sdk/src/document.rs:1411`),
`document.appearance_of` (`:2114`), `canvas.text`
(`crates/craftkit-sdk/src/document.rs:790`), `Rect::contains` (`:58`, which
takes the point **by value**), `Point::new` (`:33`), `Colour::BLACK`.

**The props are a2ui's names and a2ui's types; the component holds the
resolved values.** `TextComponent` has a `text: Text`, a
`variant: Option<Text>` and a `weight: Option<Number>` — the *resolved*
shape — while the **published** schema says:

```json
{ "component": { "const": "Text" },
  "text": { "$ref": "#/$defs/DynamicString" },
  "variant": { "type": "string", "enum": ["caption", "body"], "default": "body" },
  "weight": { "$ref": "#/$defs/Weight" },
  "required": ["component", "text"] }
```

**The discriminator and the `required` list are a2ui's**, and the compiler
writes them from the row's key — an agent that reads the catalogue sees the
same shape a2ui's own catalogue has. And the **narrowings are declared**:
a2ui's `variant` is an **enum** of `["caption", "body"]`, and the layer's
published schema keeps that enum, because a narrowing a catalogue does not
say is a narrowing an agent cannot see.

expands to — **all of it written by the compiler**, except the shapes the
specification gives, which are **re-derived** and checked (§2.2):

| emitted | what it is |
|---|---|
| `pub struct TextComponent { … }` | the **resolved** props, as a struct |
| the **`"component"` discriminator** | a2ui's own: every a2ui component's schema carries `"component": { "const": "<Name>" }` (`catalogs/basic/v1/catalog.json`) |
| the **`required`** list | from the **props**: a required prop is in it, an `Option<T>` is not, and the discriminator always is (`Text` requires `["component", "text"]`) |
| `impl TextComponent { fn read(params) -> Result<Self, EngineError> }` | the reader, from `ParamKind::read` |
| `impl TextComponent { fn doc() -> Vec<ParamDoc> }` | the declarations, from `ParamDoc::required` / `optional` |
| `impl TextComponent { fn schema() -> serde_json::Value }` | the **published** schema: the re-derived shapes, the row's names, the **local** `$ref`s (`#/$defs/…`, authored in place), the discriminator |
| `impl TextComponent { fn of(node: &Node) -> Option<Self> }` | the typed view of a node's **payload** — `node.data` |
| `pub static TEXT_COMPONENT: NodeKindSpec` | the **kind** |
| `pub static TEXT_CATALOG: CatalogEntry` | the **catalogue entry** |
| `inventory::submit!` ×2 | the registration, by the linker |

**And `schema()` is a2ui's, not craftkit's.** The schema an agent is told is
the schema **a2ui** says — the prop names a2ui uses and the shapes a2ui
defines — because that is the vocabulary the agent speaks. The craftkit
`ParamDoc`s are the *resolved* side, and they are what the SDK's own schema
mapping (Arc A) uses for craftkit's doors. **Two schemas, one row, both
written by the compiler** — and that is the whole point of the row.

**The payload is `node.data`.** A node carries its kind's own data there,
and "the core never looks inside it"
(`crates/craftkit-sdk/src/document.rs:1453-1455`) — so `of(node)` reads the
props out of the payload with the same `ParamKind::read` the commands use,
and returns `None` when the payload does not fit. A node whose payload is
missing is drawn as nothing, and that is the same as any other kind's
missing payload.

**The naming rule.** The row's key is the **a2ui component name** — `Text` —
and it is **exempt from the dotted lower-case id rule** that applies to
commands, because a2ui's catalogue names its components in PascalCase and
that name is what an agent says. From the key the compiler derives:

| the key | the struct | the statics |
|---|---|---|
| `Text` | `TextComponent` | `TEXT_COMPONENT`, `TEXT_CATALOG` |

The craftkit **kind id** is separate and is the ordinary dotted id:
`kind: "a2ui.text"`.

**And the rule for it is one rule, for every component:** the kind id is the
a2ui component name, **lower-cased**, with the layer's prefix — `Text` →
`a2ui.text`, `TextField` → `a2ui.textfield`, `Divider` → `a2ui.divider`. The id is **never** the a2ui name itself, because the
registry refuses an id that is not dotted and lower case
(`crates/craftkit-sdk/src/contribution.rs:251-265`) — and the a2ui name is
PascalCase. The a2ui name lives in the catalogue entry's `name` (§3.1), and
the two names are never the same string.

### 2.4 The single-component spelling

The derive takes the props as the struct's fields, and the painter as the
struct's methods — the same division `commands!` makes with `run:`:

```rust
/// A line of text.
///
/// The doc comment is the component's one line about itself: it is what the
/// catalogue shows and what the schema says.
#[derive(A2uiComponent)]
#[a2ui_component(name = "Text", kind = "a2ui.text", layer = "a2ui",
                 table = "craftkit_a2ui::defs",
                 shapes = "craftkit_a2ui::schema")]
pub struct TextComponent {
    /// The text to show.
    pub text: Text,
    /// A hint for the base text size.
    pub variant: Option<Text>,
}

impl TextComponent {
    /// How it is drawn.
    fn paint(node: &Node, document: &Document, canvas: &mut dyn Canvas) { … }

    /// Whether a point is inside it.
    fn hit(node: &Node, document: &Document, point: Point) -> bool { … }
}
```

**The fields are the resolved props**, so a field's type is a craftkit type —
`Text`, `Option<Text>`. **And the published schema is a2ui's**, so the derive
needs to know which of a2ui's types each field came from: `text` was declared
`DynamicString`, and `variant` was a plain `Text`. The derive therefore takes
the **a2ui definition** as an attribute on the field, exactly as `commands!`
takes `#[param(or = "…")]` on a field:

```rust
pub struct TextComponent {
    /// The text to show.
    #[a2ui_prop(DynamicString)]
    pub text: Text,
    /// A hint for the base text size.
    pub variant: Option<Text>,
    /// How much of the row this takes.
    #[a2ui_prop(Weight)]
    pub weight: Option<Number>,
}
```

**And the `table` and `shapes` attributes are *paths*, not dependencies.**
The derive is **L0** and may not depend on the layer (L3), so it cannot name
`craftkit_a2ui` by itself — and it must not: the derive would then know
a2ui. So the row names **two** paths — the **table** (the property names)
and the **shapes** (the re-derived schema text) — and everything the derive
emits resolves **where the macro is used**, in the plugin (L4), where
L4 → L3 → L2 is allowed. The derive crate stays free of a2ui, and the
**plugin** is what carries the dependency.

**And the derive's emitted `fn schema()` is a *call* into the layer's
shapes**, not a copy of them: the layer's `schema::published(name, names,
defs)` takes the component's name, the row's property **names**, and the
row's per-property **a2ui definitions** — the `#[a2ui_prop]` values — and
returns the published schema. So the derive writes **the names, the
definitions, the discriminator and the call**, and the layer owns the
**shapes** — which is the only way both can be true at once.

**And the definitions travel with the names**, because a name without its
definition says nothing: `text` is a `DynamicString` and `variant` is a
plain string, and the layer cannot know that unless the row says so. So the
row is the source of **which a2ui definition each prop uses**, and the call
carries it.

**One declaration, two schemas.** The field's **type** is the resolved
craftkit type; the field's **`#[a2ui_prop]`** names the **a2ui definition**
literally — `DynamicString`, `DynamicBoolean`, `Weight`, `Child`,
`ChildList`, `Action` — and the attribute's name says "a2ui" because the
*published* type is the a2ui one; and the doc comment is the one line the catalogue
shows. Nothing is written twice.

The derive emits the kind's spec **exactly as `expand_node_kind` does**
(`crates/craftkit-derive/src/seams.rs:200-215`) — and `paint` is a **plain
function pointer**, not an `Option`:

```rust
paint: |node, document, canvas| TextComponent::paint(node, document, canvas),
hit: |node, document, point| TextComponent::hit(node, document, point),
```

(`NodeKindSpec::paint` is `fn(&Node, &Document, &mut dyn Canvas)` —
`crates/craftkit-sdk/src/document.rs:2423`; only `evaluate`, `place` and
`part` are `Option`.)

### 2.5 What stays hand-written, and why

**The painting.** This is not a limitation of the plan — it is craftkit's own
rule, quoted from `crates/craftkit-derive/src/seams.rs:135-140`:

> *"The kind's **paint** and **hit** are the kind's own methods, and the
> macro calls them: the core never matches on a kind, so a plugin's kind is
> drawn by the plugin."*

**The boundary, stated once: the compiler writes everything mechanical; the
seam stays hand-written.** That is the division the tree already draws for
commands — the compiler writes the declaration, the reader and the
registration; the body is the author's.

### 2.6 Two catalogues, and neither is written by hand

This is where the first draft of this plan conflated two things. They are
different.

| | **the vocabulary view** | **the a2ui catalogue** |
|---|---|---|
| what | every command, tool, panel and kind — the **whole vocabulary**. A command carries its schema; a kind carries a label and no schema, because a kind's payload is its own business | the **components** — the things an a2ui agent may put on a surface — **and the functions**: the ones the layer implements and the ones it expects the agent to have (§5.1) |
| where | `Projection::catalog()` in the SDK (Arc C) | the **projection of `Projection::entries()`** — the generic view of the **`CatalogEntry` registry** — into a2ui's `catalog.json` shape, **plus the function table** (§5.1), in `craftkit-a2ui` (Arc F) |
| whose work | **O1** in `survey-a2ui.md` | **this plan** |
| an agent | may be told about it | may put these on a surface |

**And there are three views, not two.** `Projection::catalog()` is the
**vocabulary view** (Arc C, and O1's work); `Projection::entries()` is the
**generic** view of the catalogue entries (Arc B); and the **a2ui catalogue**
is the a2ui-shaped projection of the second, in `craftkit-a2ui` (Arc F). The
first two are the SDK's, and the third is the layer's — because the SDK must
not know what a2ui is.

**They are projections of two different registries, and they are not
disjoint.** The vocabulary view is a view of the **commands, tools, panels and
kinds**; the a2ui catalogue is a view of the **catalogue entries** — the
contributions this plan adds in Arc B (`Ecosystem::catalog`).

An a2ui component is **both**: a craftkit **kind** (`a2ui.text`) and a
**catalogue entry** (its published schema). So it appears in the vocabulary
view **as a kind** — with a label and no schema, because
`Projection::manifest()` publishes kinds that way
(`crates/craftkit-sdk/src/ecosystem.rs:571-573`) — and in the a2ui catalogue
**as a component**, with its published schema. **The overlap is exactly the
eleven a2ui kinds.** The two views differ in **shape** and in **reader**, not
in membership — and that is why one registry would not have merged them. So
"the vocabulary view lists commands, tools, panels and kinds **and nothing
else**" (Arc C) and "the a2ui catalogue lists **exactly the registered
components**" (Arc F) are two different sets: the first includes `a2ui.text`
**as a kind**, the second includes it **as a component**.

What the second catalogue §10 refuses is the other thing: craftkit's **other**
eighty-eight kinds are **not** a2ui components.

**No `catalog.json` is ever written by hand, and neither is the vocabulary
view.** Both are projections of the registry, exactly as the menu and the
schema already are (`crates/craftkit-sdk/src/ecosystem.rs:395-398`):

> *A menu, a toolbar, a schema, a chord map and a document are five views of
> the same registry, and this is where they are made, so that a command added
> by a plugin appears in all five without anybody writing it down five times.*

A component that is not registered is in neither; a registered one is in the
a2ui catalogue and in the vocabulary view as a kind.

**And the a2ui catalogue is the *components*, plus the functions.** Exposing
craftkit's **other** kinds there would be the second catalogue this plan
refuses (see §10). The **functions** are not components: they are the
**declaration** the protocol requires of a renderer that refuses unknown
function names (§5.1), and a `catalog_definition` — the shape both the
served catalogue and an inline one have — has a `functions` map.

**And the map has exactly two kinds of entry** (§5.1): the functions the
layer **implements**, and the agent-side functions it **expects but does not
implement**. Both are declared `"rendererOnly"`, and both are written by one
**function table in `craftkit-a2ui`**.

---

## 3. The crates and the layers

| crate | layer | what it is | new? |
|---|---|---|---|
| `craftkit-sdk` | L2 | `CatalogEntry`, `Projection::catalog()`, the `Doc` → JSON Schema mapping, **the SVG path-data reader** | **changed** |
| `craftkit-derive` | L0 | `#[derive(A2uiComponent)]` and `components! { … }` | **changed** |
| `craftkit-a2ui` | L3 | the a2ui protocol types, the data model, the binding resolution, the box layout, the a2ui catalogue projection | **new** |
| `craftkit-plugin-a2ui` | L4 | the components: kinds, painters, catalogue rows, and the commands the door runs | **new** |
| `craftkit-plugin-vector` | L4 | its SVG path-data **reader**, which moves down to the SDK; the importer stays and calls it (Arc E) | **changed** |
| `apps/craftkit-a2ui-server` | L5 (app) | the door: stdio JSON-RPC speaking a2ui's messages | **new** |

**And the two are not both `craftkit-a2ui`.** The library is
**`craftkit-a2ui`** (L3) and the door is **`craftkit-a2ui-server`** (L5) —
the tree's own convention, which is `craftkit-mcp` and `craftkit-mcp-server`.
It has to be: the workspace's members are globs (`Cargo.toml`'s
`members = ["crates/*", "apps/*", …]`), and **Cargo refuses two members with
the same package name** — so two crates called `craftkit-a2ui` would not
build at all. And the gate resolves crates **by package name** across
`crates/` and `apps/` (`crates/craftkit-gates/src/lib.rs:897-918`) and
matches `FRONT_ENDS` on that name (`:631`), so a duplicate would also put
the **library** under the thin-front-end rule.

**Layering checks** (`crates/craftkit-gates/src/lib.rs:437-491`):

- `craftkit-a2ui` (L3) → `craftkit-sdk` (L2). ✔
- the **SVG path-data reader** in the SDK (L2) → nothing below. And the
  vector plugin (L4) → the SDK (L2) ✔ — which is how the plugin and the
  a2ui layer come to share one reader (§4.3).
- `craftkit-plugin-a2ui` (L4) → `craftkit-a2ui` (L3), `craftkit-sdk` (L2). ✔
  The plugin rule forbids plugin→plugin and plugin→app; neither applies.
- `apps/craftkit-a2ui-server` → everything. Apps are exempt. ✔
- `craftkit-derive` (L0) → nothing. It emits code, it does not call it. ✔

### 3.1 The five edits a new contribution type really needs

`CatalogEntry` is a contribution, and a new contribution type is not free. The
SDK edits are:

1. `inventory::collect!(crate::contribution::Registration<CatalogEntry>);`
   — beside the others (`crates/craftkit-sdk/src/seams.rs:46-48`);
2. `pub catalog: Registry<CatalogEntry>` on `Ecosystem`, beside the others
   (`crates/craftkit-sdk/src/ecosystem.rs:112-120`), and `Registry::new()` in
   its constructor (`:133-137`);
3. a `from_inventory` arm in `Ecosystem::from_inventory` (`:148-157`);
4. a `check_layers` arm (`:287-295`) — **every contribution's layer must
   exist in the layers registry**, so the `a2ui` layer needs a `LayerSpec`;
5. a **`Projection::entries()`** method over it — a **generic** JSON view of
   the catalogue entries, with no shape of its own. It lives beside the other
   projections, in `crates/craftkit-sdk/src/ecosystem.rs` — **not** in
   `contribution.rs`, where the *type* is — and it is Arc **B**'s second port
   row. The **a2ui-shaped** projection is **Arc F's**, and it lives in
   `craftkit-a2ui` (L3), because the L2 SDK must not know what a2ui is.

And `CatalogEntry` must satisfy the `Contribution` trait, whose `capability()`
has **no default** (`crates/craftkit-sdk/src/contribution.rs:48-53`):

```rust
/// A **catalogue entry**: a named, schema-described thing an agent may use.
#[derive(Debug, Clone)]
pub struct CatalogEntry {
    /// The entry's id: dotted, lower case, unique.
    ///
    /// It is craftkit's own name, and it is the one the registry checks:
    /// an id that is not dotted and lower case is refused
    /// (`crates/craftkit-sdk/src/contribution.rs:251-265`).
    pub id: &'static str,
    /// The name it is **published under**: the key in the catalogue.
    ///
    /// It is usually the id, and it is different when the vocabulary the
    /// entry is published in names it differently — an a2ui component is
    /// `a2ui.text` here and `Text` there.
    pub name: &'static str,
    /// What it is called, for a person.
    pub label: &'static str,
    /// The layer that owns it.
    pub layer: &'static str,
    /// One line about itself.
    pub doc: &'static str,
    /// Its schema: the parameters, as JSON Schema.
    pub schema: fn() -> serde_json::Value,
}

impl Contribution for CatalogEntry {
    fn id(&self) -> &'static str { self.id }
    fn capability(&self) -> &'static str { "component" }
    fn layer(&self) -> &'static str { self.layer }
}
```

**Why `name` is a field and not the `id`.** The registry refuses an id that
is not dotted and lower case, and a2ui's component names are `Text`,
`Button`, `ChoicePicker` — PascalCase, and **not** craftkit's. So the entry
has two names: `id`, which is craftkit's and is checked, and `name`, which is
the **published** one and is the key the catalogue uses. A catalogue entry for
a component that is not an a2ui component would have `name == id`.

**And `Debug` is required**, because `Registry<T>` derives it
(`crates/craftkit-sdk/src/contribution.rs:138`).

**And the entry's `schema` is the *published* one** — a2ui's shape (§2.2).
The craftkit-side schema — the one Arc A's moved mapping builds from the
`ParamDoc`s, and the one craftkit's own doors are told — is the component's
`doc()`. Two schemas, two readers, neither written by hand, and the entry
carries the one the agent needs.

**The `a2ui` layer** is declared by the plugin, with the same derive the other
batteries use:

```rust
/// The a2ui compat layer.
#[derive(Layer)]
#[layer(id = "a2ui", name = "A2UI", requires("core"))]
pub struct A2uiLayer;
```

(`LayerSpec` has **`name`**, not `label` — `crates/craftkit-sdk/src/seams.rs:103-110`
— and `name` is its human-facing title, while `id` is the dotted id the layer
registry is keyed by. And `requires("core")` is not decoration: every plugin
layer in the tree declares it (`crates/craftkit-plugin-art/src/lib.rs:70`), and
a layer with no requirements would be a **second root** of the runtime layer
graph.)

---

## 4. Which components, and why those

a2ui's basic catalogue has **eighteen** components: `Text`, `Image`, `Icon`,
`Video`, `AudioPlayer`, `Row`, `Column`, `List`, `Card`, `Tabs`, `Modal`,
`Divider`, `Button`, `TextField`, `CheckBox`, `ChoicePicker`, `Slider`,
`DateTimeInput`.

craftkit's `Canvas` has **four** methods (`crates/craftkit-sdk/src/document.rs:776-801`):
`rect`, `text`, `path`, `picture`. So craftkit can draw a box, a line of text,
a path and a picture — and **nothing else**.

| a2ui | paintable? | why |
|---|---|---|
| `Text` | **yes** | `text` and `variant` → `canvas.text` |
| `Divider` | **yes** | `axis` → a `rect` |
| `Card` | **yes** | a `rect` and its `child` |
| `Row` | **yes, with work** | `children`, `justify`, `align` — a box arrangement, and craftkit has **no** layout engine (§4.1) |
| `Column` | **yes, with work** | the same, down the other axis |
| `List` | **yes, with work** | `children`, `direction`, `align` — a column of children |
| `Button` | **yes, with work** | a `rect`, its `child` (usually a `Text`), and an `action` — and the action is the button's whole point (§5) |
| `CheckBox` | **yes, with work** | a `rect`, a `path` mark, a `label` and a `value` that is a **boolean**, so the layer must turn a change of it into a command |
| `TextField` | **yes, with work** | a `rect`, a `label`, a `placeholder`, and a **value the user supplies** — and the value is a **bound path in the layer's data model**, not a question craftkit asks (§5.2). **And the first cut has no keyboard**, so the value comes from the agent or from a control-channel `Execute`, not from typing (§5.2) |
| `Image` | **yes, with work** | a2ui's image is a **URL**, and **nothing in the tree fetches one** — there is no HTTP client in any manifest. The bytes must come from somewhere the plan has to name, and §4.2 says where |
| `Icon` | **yes, with work** | a2ui's `Icon.name` is one of **59 built-in names** or an object with a custom `svgPath`; craftkit has **no icon vocabulary at all**, so only the custom path is painted (§4.3) |
| `Video`, `AudioPlayer` | **no** | no media in the seam — and no decoder |
| `Tabs`, `Modal`, `ChoicePicker`, `Slider`, `DateTimeInput` | **no** | not because they cannot be **drawn** — a slider is two rectangles, and a choice picker is a list of them — but because their **interaction** is a widget's: a value the user drags, a list the user opens, a tab that is chosen. craftkit's seam has no notion of any of those, so they are **refused** rather than faked (§4.4) |

**Eleven components to start with**, of which **eight need work** the plan names
(`Row`, `Column`, `List`, `Button`, `CheckBox`, `TextField`, `Image`, `Icon`).
The other three — `Text`, `Divider`, `Card` — are a `rect` and a `text`.
The rest are **refused, not faked** — the house rule, and a2ui's own answer
(the `unknown-type` placeholder) is the thing not to copy.

**The count is eleven and not ten**, because `Row` and `Column` are **two**
components in a2ui's catalogue, and a plan that counts them as one is
counting a layout, not a component.

**The props above are a2ui's**, and the layer's props are the **resolved**
ones (§2.2). The **mapping is per component and is hand-written**, because it
is the one thing no macro can write: which a2ui prop becomes which craftkit
prop is a decision, not a derivation.

**And every a2ui component carries a `weight`** — a number, the relative
weight of the component inside a `Row` or a `Column`, like CSS's `flex-grow`
(`catalog.json`'s `$defs/Weight`). So **every** component's published schema
carries it, and the box layout (§4.1) is the thing that reads it. A component
that leaves it out is a component that cannot be arranged.

### 4.1 The one thing craftkit does not have: a box layout

`Row`, `Column` and `List` need to **arrange children**. craftkit has:

- `craftkit-text-layout` — **line breaking** (Knuth–Plass), for paragraphs;
- `craftkit-font` — the metrics of a typeface;
- **no** flex, no box arrangement, no stack.

**Decision: the layout lives in `craftkit-a2ui` (L3).** Not in the plugin.
The reason is the plugin rule: `craftkit-plugin-a2ui` may not depend on
another plugin, so a layout in a plugin could never be shared with a second
one; and L3 is exactly "the things above the SDK that are not plugins".

**And it is not general.** It is the minimum arrangement this layer needs:
measure the children, stack them. The trigger for making it more general: the
moment a second renderer needs to arrange the same children.

**And the arrangement is computed at apply time, and a command writes it.**
The `place` seam cannot carry it: `place` returns **one** rectangle for
**one** node — `Option<fn(&Node, &Document) -> Option<Rect>>`
(`crates/craftkit-sdk/src/document.rs:2448`) — and so it cannot arrange
several children. `Session::place_of` (`crates/craftkit-sdk/src/session.rs:317`)
**does** have callers — the bounds and the selection frame (`:349`, `:449`) —
but it only ever asks a kind for **its own** node's rectangle. And the paint
path reads `node.rect` (`:1411`), not the seam.

So the arrangement is worked out **when the surface is applied**.
`a2ui.surface.apply` (Arc H) takes the component tree, the layer **measures
the children and stacks them**, and it creates each child node **with its
rectangle** and **under its parent** —
`Document::add_node_with_under(parent, name, kind, rect, data)`
(`crates/craftkit-sdk/src/document.rs:1748-1774`) — **through the command**,
never by writing the document outside one. (The flat `add_node_with` would
put every node at the **top level**, and a surface is a **tree**: the
container nodes hold their children, so the containment is in the tree and
the layout is in the rectangles.) So:

- the rectangles are the children's **places**, set by the command that
  creates them, exactly as every other node's place in craftkit is set by the
  command that creates it;
- the layout is **derived from the tree** every time the tree is applied, and
  **no layout is stored**;
- a child whose place the layout cannot work out is a node with **no place** —
  which the house rule allows, and which no painter has to draw.

### 4.2 Text is drawn as boxes, and images are not fetched

- **Text.** The rasteriser has **no font**: *"the kit has the metrics of a
  typeface and not its glyphs, so text is drawn as the boxes it would occupy:
  a typewriter's worth of rectangles, one per character"*
  (`crates/craftkit-raster/src/lib.rs`). The compat layer inherits this. An
  a2ui `Text` will be a row of rectangles in the raster and the window —
  **honest, and exactly as honest as craftkit already is.** Not a defect of
  the layer, and not fixed here.
- **Images.** a2ui's `Image` is a URL. The tree has **no fetcher**, so the
  first cut takes the bytes from a **file** the surface names, and the
  catalogue's `Image` entry says so — an agent is told the URL is a path, not
  a promise to fetch. And, as with `Icon`, the **published schema says so**:
  the `url` property's description says it is a path, so the narrowing is in
  the schema the agent reads and not only in the prose. a2ui's `Image` also
  carries `fit` and `variant` enums, and the published schema keeps them —
  a narrowing the catalogue does not say is a narrowing an agent cannot see.
  A real fetcher is a separate decision, with the dependency consequences §9
  names.
  The **decoder** is the tree's: `craftkit-png` reads a PNG
  (`crates/craftkit-png/src/lib.rs:83`). **And there are two `Picture`
  types** — `craftkit_png::Picture` holds `width`, `height`, `channels` and
  `samples`, while `craftkit_sdk::picture::Picture` — the one `Canvas::picture`
  paints (`crates/craftkit-sdk/src/document.rs:800`) — holds `width`, `height`
  and `pixels`, four bytes a pixel. So there is a **conversion**, and the
  image battery already writes it: `Picture::from_samples(width, height,
  channels, samples)` (`crates/craftkit-sdk/src/picture.rs:41`, used at
  `crates/craftkit-plugin-image/src/lib.rs:244`).
  **Who decodes: the plugin's painter**, and it reads the file with
  `craftkit-png` (**L1**, so an L4 plugin reaches it) — exactly as
  `craftkit-plugin-image` does. The **layer** does not decode anything.
  **And the read is bounded, because the path comes from outside.**
  `craftkit-png`'s `read` takes a `Limits`
  (`crates/craftkit-png/src/lib.rs:83`), and that limit is
  `craftkit_deflate::Limits { max_output }` — which bounds the **inflated
  stream**, **not** the file. The bytes themselves come from reading a file,
  and a file's length is an **input-derived number** (the house rule: *"input-
  derived numbers are hostile … bound sizes"*), so the layer **checks the
  length before it reads** and refuses a file over a stated maximum — a
  maximum it writes down, not one it inherits. A PNG over it is **refused**,
  not decoded and not truncated.

  **And because the picture is read at paint time, it is read again on every
  frame** — so the check runs on every frame too, and the first cut does
  **not** cache the picture. The cost is a file read per picture per frame,
  which is what `craftkit-plugin-image` already pays.

  **And the decoded picture is not stored**: the node carries the **path**,
  and the picture is read at paint time — the tree's own way, because a
  document that kept the pixels of every picture would keep a copy of every
  file it ever showed. And the node's payload holds **only the path**: the
  picture's width and height are **derived** — they are measured from the
  file when the painter runs — and a derived value is not stored
  (`crates/craftkit-plugin-image` stores them in its payload, and that is
  that plugin's business, not this layer's).
  A URL that is not a file, or a file that is not a PNG, is **refused** — not
  fetched, not guessed at.

### 4.3 The icon vocabulary craftkit does not have — and the parser it cannot reach

a2ui's `Icon` takes a `name`, and the name is one of **three** things
(`catalogs/basic/v1/catalog.json`):

- one of **59 built-in names** — `accountCircle`, `add`, `arrowBack`, …;
- an **object with `svgPath`** — a custom SVG path;
- a **data binding**.

craftkit has **no icon vocabulary at all**: there is **no icon node kind** in
the tree, no icon set, and no icon file of any kind. An icon in craftkit is a
*name on a command or a tool* (`CommandSpec.icon`, `ToolSpec.icon`), and
"the drawing is the front end's business" — so the names exist and the
drawings are in code, per front end.

So the **built-in names** are not paintable without drawing 59 glyphs in code,
and that is a large hand-written body that no macro can carry — and copying
59 paths out of another project would be copying artwork, which this tree does
not do. **Decision: the layer's `Icon` accepts the custom-`svgPath` form only,
and refuses the built-in names.** The **published schema says so, exactly**:
it is a2ui's `Icon` schema with **the 59-name branch taken out** — the `name`
property is still a `oneOf` of the object with `svgPath` and a data binding,
and the enum of built-in names is **gone**, so an agent that reads the
catalogue sees the narrowing **before** it sends anything. A built-in name is
refused with a clear error rather than drawn as a box. This narrows a2ui's
component; it does not narrow craftkit's own UI, which has no icons to
narrow.

**The custom path is paintable — but the parser is not reachable.** This is
the part the first draft of this plan got wrong, and it cost more than the
icon.

| what | where | can the layer reach it? |
|---|---|---|
| `craftkit-plugin-vector`'s `read_path` (`src/svgimport.rs:110`) — a reader that produces the SDK's `Path` | **L4**, a plugin | **no** — a plugin may not depend on another plugin |
| `craftkit-plugin-cad`'s own reader (`src/svgimport.rs:242` — `ops_of`, `runs_of`, `cubic_at`) — a **second**, over the CAD battery's own types | **L4**, a plugin | **no** — the same rule, and it is a *different* implementation |
| `craftkit-vector-geom` | **L3** | **no** — and it holds no reader at all: it is the **operations** on a shape (`boolean`, `offset`, `simplify`, `outline_stroke` — `crates/craftkit-vector-geom/src/lib.rs:255-680`), not a parser of path data |
| the SDK's `SubPath` and `Path` (`crates/craftkit-sdk/src/document.rs:613`, `:651`) | **L2** | **yes** — but they are **shapes**, not readers: the data a reader would produce |

So the layer cannot read an SVG path with anything the tree already has.
**And the tree has *two* readers, not one** — the vector plugin's and the
CAD plugin's — so the plan's claim is **"one reader for the a2ui layer"**,
not "one reader in the tree". The CAD battery's reader is a **separate**
implementation over the CAD battery's own types, and **it stays**: the plan
does not touch the CAD battery, and unifying two readers that were written
apart is a different piece of work.

**Two obligations follow, and both are real:**

1. **A parser.** Either the SVG path-data reader is **promoted to L2** — into
   the **SDK**, as `crates/craftkit-sdk/src/svgpath.rs` — so that the vector
   plugin and the a2ui layer share one, or the layer **writes its own** and
   there are two readers in the tree. The plan takes the promotion: one reader,
   one set of rules, and the plugin already depends on the SDK. The cost of the
   promotion is that the **reader** — `pub(crate) fn read_path`
   (`crates/craftkit-plugin-vector/src/svgimport.rs:110`) — **moves to the
   SDK**, and the importer stays where it is and calls it. The importer's own
   API does not move: it reads a file, and that is still its job. The **CAD**
   plugin's reader is **not** promoted and **not** touched: it is a different
   implementation over the CAD battery's own types, and the plan's promise is
   **one reader for the a2ui layer**, not one reader in the tree.
2. **An arc.** The tree's reader **refuses arcs** — *"an arc is not read, and
   a file that holds one is refused: an arc is a curve, and this tree does
   not write a curve it cannot say"*
   (`crates/craftkit-plugin-vector/src/svgimport.rs:105-109`) — and SVG path
   data **may** carry elliptical arcs (`A` and `a`), which a2ui does not
   constrain. So the reader needs an **arc**, which means an arc-to-cubic
   conversion, which means the tree can finally say a curve it cannot say
   today.

**And the decision is the house rule's:** an icon whose path holds an arc the
reader cannot convert is **refused**, with the reason, and the catalogue says
which commands the reader accepts. Not a silent omission, and not a shape
drawn wrongly.

### 4.4 The components that are refused, and why that is not "no"

`Tabs`, `Modal`, `ChoicePicker`, `Slider` and `DateTimeInput` **can** be
drawn — a slider is a track and a thumb, and a choice picker is a list of
labels. What they need is an **interaction**: a value the user drags, a list
the user opens, a choice that is chosen. craftkit's `Canvas` seam draws; it
does not model a widget's state, and craftkit's document has no node kind
that means "the user is dragging this".

So the refusal is not "craftkit cannot draw it". It is:

> **craftkit does not know what these mean**, so it refuses them rather than
> drawing a picture of a widget that does nothing.

That is the house rule — *unknown names are refused* — and it is the same
rule that made the plan refuse a2ui's `unknown-type` placeholder. Each of the
five is a **declaration** waiting for an application that knows what its
`Slider` means, and an application that declares one gets a component of its
own — which is §10's "an application that wants `layout.spread` addressable
declares that itself".

---

## 5. The data model and the bindings

a2ui's props are `DynamicString` and friends, and a `DynamicString` is **one
of three things** (`common_types.json`'s `$defs/DynamicString`):

- a **literal** — a string;
- a **binding** — `{"@path": "/title"}`, a JSON Pointer into the data model;
- a **function call** — a call the renderer makes to the agent.

`DynamicBoolean` and `DynamicNumber` are the same three ways.

**The data model's type lives in `craftkit-a2ui`**, and its **value lives in
the document**:

- the crate holds a JSON tree, addressed by JSON-Pointer-ish paths;
- the **surface's** model is in the **surface's node**, under `model` — so it
  is undone with the document, it is not shared with another surface, and it
  is not shared with another session;
- a2ui's `updateDataModel` writes it, **through a command**;
- a binding is resolved in the **painter**, because the model is in the
  document and the painter is the only place that has both — so the props
  the component sees are the values **as they were sent**, and the generated
  reader is used unchanged.

**The three forms resolve three ways, and the third one is a round trip.**

| the prop is | the layer does |
|---|---|
| a **literal** | uses it |
| a **binding** | looks the path up in the data model; a path that is not there is **refused** |
| a **function call** | resolves it in the protocol's order — **local first, the agent only as a fallback** — with §5.1's narrowing for a name the catalogue does not declare (§5.1) |

The third is the only one that can leave the process, and it is why the door
has both directions (§6).

### 5.1 A function call: local first, the agent as a fallback

The protocol is explicit about the order
(`specification/v1_0/docs/a2ui_protocol.md:1482-1486`):

1. **Catalogue resolution** — the call's catalogue, from its own `catalogId`
   or the surface's default. A call whose catalogue cannot be resolved is a
   **catalogue error**, and it is **not** routed to the agent.
2. **Local lookup** — a reserved `@`-prefixed function is **always** local and
   **must not** go to the agent; otherwise the layer looks for a **local
   implementation** in that catalogue.
3. **Fallback** — **only** if there is no local implementation, the layer
   dispatches **`callAgentFunction`**, and the answer comes back as
   **`agentFunctionResponse`**.

**The layer's local implementation is a function table**: a map from an a2ui
**function name** to a craftkit **command id**. The names are the two
vocabularies' own — a2ui's functions are **UAX #31** names (`length`,
`required`, `formatCurrency` — `a2ui_protocol.md:546-548`), and craftkit's
commands are **dotted** ids (`a2ui.surface.apply`) — so the table's key and
its value are **never** the same string.

**And the table is not empty: it is the layer's own commands.** The first
cut's table holds one entry per command the layer exposes, named in a2ui's
style:

| the a2ui function name | the craftkit command |
|---|---|
| `surfaceCreate` | `a2ui.surface.create` |
| `surfaceApply` | `a2ui.surface.apply` |
| `surfaceDelete` | `a2ui.surface.delete` |
| `dataSet` | `a2ui.data.set` |
| `componentInvoke` | `a2ui.component.invoke` |

**And the three vocabularies are three namespaces.** An a2ui **function
name** (`surfaceApply`), an a2ui **message name** (`createSurface`) and a
craftkit **command id** (`a2ui.surface.apply`) are three different strings in
three different places, and the plan keeps them apart on purpose: a function
name that reads like a message name would be ambiguous to an agent
generating `{"@call": "…"}`.

**So a button's `functionCall` runs a craftkit command**, through
`Session::execute`, with craftkit's undo, its journal, its `mutates` rules
and its refusals — for free, because it used the same door. And where the
layer runs a command **inside** another command, it runs it inside
**`Session::batch`** (`crates/craftkit-sdk/src/session.rs:1083`), so the
inner command does not push a **second** history entry and the whole thing
is one undo step.

**And there is exactly one rule for a name, in three cases:**

| the name | what happens |
|---|---|
| in the **local table** | it is a craftkit **command id**, and it runs through `Session::execute` |
| **declared agent-side** in the catalogue's `functions` | it is dispatched to the agent as `callAgentFunction` |
| **neither** | it is **refused** |

**And the third row is a narrowing, and it is deliberate.** The protocol's
own default is *fallback* — a name the catalogue does not declare at all is
assumed to be the agent's (`a2ui_protocol.md:1484`). craftkit's rule is that
an **unknown name is refused**, so the door refuses it rather than sending it
on. The protocol allows the narrowing; the house asks for it.

**A reserved `@`-prefixed function is refused, not forwarded.** The protocol
says it is always local and must never go to the agent, and the layer
implements no reserved functions — so a call to one is **refused**, with the
reason. **And that is a narrowing, and it is declared as one**: the protocol
says a reserved function is *"always evaluated locally"*, and the layer
evaluates none of them locally.

**And the `ChildList` template branch goes with it.** a2ui's `ChildList` is a
`oneOf` of a **static list of component ids** and a **template** — an object
naming a `componentId` and a `path`, from which a list is rendered per item
with the reserved **`@index`** function. The layer refuses every reserved
function, so it cannot execute a template — and a template it cannot execute
is a branch it must not publish. So the **published** `ChildList` keeps the
**static** branch and the template branch is **removed**, with the reason
stated, exactly as the `Icon` enum branch is removed.

**And the table's boundaries are the catalogue's.** The protocol says a
declared function's `allowedCallers` and `requiresUserActivation` decide
whether the call may be dispatched to the agent at all, and a call that
breaks them is an `EXPRESSION_ERROR` and **is not sent**
(`a2ui_protocol.md:1484`). **`requiresUserActivation: true`** means the call
may only go out when the user **activated** it — so a binding or a check may
never dispatch it, and only an **action** may. The layer declares none of
its functions with it, and the rule is in the code because the protocol puts
it there. The layer's catalogue declares the functions it
expects the agent to have, each with **`allowedCallers: "rendererOnly"`** —
the default, and the value that *permits* the renderer's own dispatch
(`a2ui_protocol.md:1484`; `a2ui_custom_functions.md:71`) — and the layer
enforces the boundary before it dispatches.

**And `agentOnly` is the other way round, and it is worth being exact about.**
`allowedCallers` says **who may call the function**: `"rendererOnly"` (the
default) lets the **renderer** call it and forbids the **agent** from calling
it with `callRendererFunction`; `"agentOnly"` is for a function **only the
agent** may call — so the renderer **must not** dispatch it, and reports an
`EXPRESSION_ERROR` instead. A schema-only declaration the renderer intends
to fall back on is therefore **`rendererOnly`**, and a declaration meant for
the agent to call is `agentOnly`.

**And the protocol's last rule binds the catalogue**: *"a renderer that
rejects payloads naming functions absent from its catalogues MUST declare the
agent-side functions it expects, without implementing them"*
(`a2ui_protocol.md:1488`). So the declaration is **in the catalogue's
`functions` map** — not in the capabilities, which have no such field
(`renderer_capabilities.json` has exactly `supportedCatalogIds` and
`inlineCatalogs`) — and that declaration is what makes the **second** row of
the table above possible: without it, the door would have to refuse every
name it does not implement.

**And the map has exactly two kinds of entry**, and it has one producer:

| the entry | what it says | `allowedCallers` |
|---|---|---|
| a **local** function | one the layer **implements** — the five above | `"rendererOnly"` |
| an **agent-side** function | one the layer **expects the agent to have** and does not implement | `"rendererOnly"` |

**Both** are `"rendererOnly"`: that is the value that lets the **renderer**
dispatch the call, and it only forbids the **agent** from calling it back
(`a2ui_protocol.md:1484`; `a2ui_custom_functions.md:71`).

**And the map's shape is a2ui's**, because a2ui validates it:
`catalog_definition.json`'s `FunctionDefinition` requires a **`@call`**
envelope — `type`, `properties` with a `"@call"` `const`, and a `required`
holding `"@call"` — plus a **`returnType`** from a2ui's enum of eight, with
`unevaluatedProperties: false`. The `"@call"` `const` **is the entry's key** —
the function's name — and the `returnType` is the shape the function returns,
which for a command is **whatever the command returns**: so the layer's table
carries it, and a function whose return shape is not known is not
declared.

**And one producer writes it**: a **function table in `craftkit-a2ui`**
(`src/functions.rs`), compiled from the same declaration as the local
function table — because the map cannot be a projection of the
`CatalogEntry` registry, which holds **components**, not functions.

### 5.2 The value a user types

a2ui has **no question**, and no message to answer one. A `TextField` is a
**bound path**: the user types, and the renderer *"immediately updates the
value at the bound path in the local Data Model"*
(`a2ui_protocol.md:995`) — the path is created if it is not there.

So the value's home is **the layer's data model**, and it reaches craftkit
the way every other value does — **as a command**:

1. the user types, and the host writes the value;
2. the layer updates its data model at the bound path;
3. the layer **mirrors** the value into craftkit by issuing **`a2ui.data.set`**,
   and the document changes through the command, with the undo and the journal;
4. the layer sends the **`action`** to the agent, with the event's `context`
   carrying the new value.

**And a `TextField` whose `value` is a *literal* rather than a binding is
refused**: there is no path to write it to, and a value with nowhere to live
is not a value.

**And the published schema says so**, because a narrowing a catalogue does
not say is a narrowing an agent cannot see (§2.2). The **`TextField`'s
`value`** keeps only the **`DataBinding`** branch — not the literal and not
the call, for the same reason: a literal has nowhere to live, and a call's
answer has nowhere to live either. So it is the **fifth narrowing**, and
the reason is the same one.

**And the first cut has no keyboard, and that is a narrowing.** The control
channel's method set is **closed** — `Execute`, `Describe`, `State`,
`Diagnostics`, `Ping`, `Shutdown`, `UiRender`, `UiClick`, `UiDrag`
(`crates/craftkit-control/src/lib.rs:104-123`) — and **not one of them
carries text**. So a `TextField` in the first cut **displays** its bound
value, and the value is set by the **agent** (an `updateDataModel` message)
or by a control-channel **`Execute`** of `a2ui.data.set`. Typing needs a
front end with a keyboard, and the door has none — so the **narrowing is
declared**: the component is published whole, and what the first cut cannot
do is **type into** it.

**And this is where an earlier draft of this plan was wrong.** It said craftkit
would *ask* for the value, through `doc.ask`
(`crates/craftkit-sdk/src/ask.rs:63`). It would not work over this door:
`doc.ask` (`crates/craftkit-sdk/src/ask.rs:64`) is a command whose answer
comes back through craftkit's **resume** seam, and a2ui has no message for it — the agent→renderer direction is
exactly six messages and the renderer→agent exactly four (§6), and **none of
them is a question**. So the value comes from the **front end** (§5.3), and
craftkit's side of it is a command like every other.

### 5.3 How input reaches the layer

**The layer never handles input.** craftkit's own rule is that a gesture
**runs a command** — "a tool is a gesture that runs a command"
(`crates/craftkit-sdk/src/seams.rs:28-41`) — and an a2ui surface obeys the
same rule. So:

| the user does | the front end turns it into | the layer does |
|---|---|---|
| **clicks** a component | **`a2ui.component.invoke`** — the component's id and the gesture | the layer runs the command and the surface updates |
| **types** into a `TextField` | **`a2ui.data.set`** — the bound path and the value | the layer writes the value into its model and mirrors it into craftkit |

**And the mapping from a click to that command is the *door's* work, and it
is named here because it is not free.** The tree's only click path is the
window's pointer code, and it ends in `select.node`
(`apps/craftkit-window/src/lib.rs:534-557`) — a **craftkit** command. A kind's
`hit` only **reports** a hit (`crates/craftkit-sdk/src/document.rs:2409-2455`);
nothing in the tree maps a hit on an `a2ui.*` kind to a command. So the door
writes its own **hit-test** — walk the document's nodes, call
`(kind.hit)(node, document, at)`, and if the hit node's kind is an `a2ui.*`
one, run **`a2ui.component.invoke`** with that node's id. It is the window's
pointer logic with a2ui's last step instead of `select.node`, and it is
**Arc I's**.

**And the door's surface is drawn through the **control channel** — because
every app has its own session.** `apps/craftkit-a2ui-server` is a **stdio
JSON-RPC app**, like `craftkit-bridge`: it speaks a2ui and it **opens no
window**. And each app builds **its own** ecosystem and session
(`apps/craftkit-bridge/src/main.rs:11-18`), so the door's document is **not**
the window's document and nothing shares it.

So the door **serves** its own session on a **control channel**:
`craftkit_control::Server::serve_shared` (`crates/craftkit-control/src/lib.rs:440`)
serves a shared session, and the channel's methods include **`UiRender`**,
**`UiClick`** and **`UiDrag`** (`:104-125`) — "what the program is showing, as
a picture" and "click in the document".

**And the channel needs a `Render`, and the door must supply one.**
`craftkit_control::Render` (`:80-96`) has three methods — `render`, `click`
and `drag` — and a server **without** one answers `"no-window"` and *"this
program has no window to show"* (`:579-581`, `:608-611`, `:636-642`). The
only implementation in the tree is the window's.

**But the window's implementation needs no window.** It is three lines
(`apps/craftkit-window/src/lib.rs:505-517`): a `craftkit_raster::Raster`, a
`session.paint_view(&mut raster)`, and `raster.image().to_png()`. The
rasteriser is **software**, so the door can do exactly the same — which is
how the door's surface becomes **visible** with no screen anywhere.

**And the first cut is therefore **headless for a human**.** A window that
*attaches* to somebody else's session is **not in the tree today** — the
window serves its own channel (`apps/craftkit-window/src/main.rs:121-132`) —
and this plan does not add one. What ships is a door an agent can drive and a
control-channel client can see; **the door is not a window**.

**And that is the whole of the input path**, because the commands are
commands: the layer's own (`a2ui.component.invoke`, `a2ui.data.set`) go
through `Session::execute` like every other, so the undo, the journal, the
`mutates` rules and the refusals are the tree's own.

**Writes into craftkit are commands, not data.** The crate mirrors a value
into craftkit by issuing a command; it never writes the document **outside a
command**. So:

- **the core does not depend on a2ui** — the SDK knows nothing about it, the
  derive gains one a2ui-specific macro that emits no a2ui *type* — it emits a
  path into the a2ui crate, and a check, and nothing else — and a binary
  without the plugin has no `a2ui.*` kinds;
- the survey's §3.4 rule is kept: no front end ever holds a `Plan`.

**And which side is the truth is said once, because it is not obvious.**
a2ui's own rule is that *"the local Data Model is the single source of
truth"* (`a2ui_protocol.md:999`), and craftkit's is that *"derived values
are derived — never stored"*. So:

| | the truth for | read from |
|---|---|---|
| **the layer's model** | an **a2ui binding** — a path in a2ui's own vocabulary | the layer's tree |
| **craftkit's document** | craftkit's own state — the geometry, the commands | the session |

**Neither is a copy of the other.** The layer **mirrors** a value into
craftkit by **issuing a command**, and the mirrored value is an **effect**, not
a second truth: it is re-derived from the model whenever the model changes,
and it is never read as authority. And the layer reads craftkit's state — it
never writes it. So the layer's tree is **a view of a2ui's own values** and
craftkit's document is **craftkit's**; the two are kept in step by commands,
and by nothing else.

**An a2ui action is one of two things** (`common_types.json`'s `Action`): an
**agent-side `event`**, or a **renderer-side `functionCall`**.

- A **`functionCall`** is resolved by the protocol's own order (§5.1): the
  layer's **function table** first, and the **agent** only as a fallback. A
  name in the table is a craftkit **command id**, and the crate runs it
  through `Session::execute` — so an a2ui surface gets craftkit's undo, its
  journal, its `mutates` rules and its refusals for free. A name that is not
  in the table goes to the agent as `callAgentFunction`; a name that is
  **neither** in the table nor in the catalogue's `functions` is **refused**.
- An **`event`** is **not** a craftkit command: it is the **agent's** own
  vocabulary — a name, an optional `userMessage` and an optional `context` —
  and the layer **forwards** it to the agent as a2ui's `action` message, the
  door's one outbound message that is not a reply to a call.

**So "every action is a command" is a narrowing, and it is stated here.** The
layer supports the `functionCall` form and carries the `event` form through
untouched. An a2ui `Button` carries an **`action`**, not a command id, and the
layer maps one form onto a command and the other onto a message.

---

## 6. The door and the messages

`apps/craftkit-a2ui-server` is a stdio JSON-RPC door, like `craftkit-bridge`.

**The door's own first request** is **`initialize`**, and it is the handshake
that carries the capabilities (§6.1). It is the one method name that is not
a2ui's.

**And the rest of `createSurface` is handled or refused, on purpose.**
`sendDataModel: true` asks the renderer to put the surface's whole data
model in the **metadata of every message** it sends for that surface
(`agent_to_renderer.json:34-38`) — and the door **does** that, because it
holds the model. And `metadata.extensions` — a2ui's map for data craftkit
does not know — is **refused**: a payload the tree cannot interpret has
nowhere to go, and the house rule is that an unknown name is refused, not
parked.

**The agent → craftkit direction** (`specification/v1_0/json/agent_to_renderer.json`):

| message | what craftkit does |
|---|---|
| `createSurface` | makes a surface, and the document node it draws into |
| `updateComponents` | the component tree → the layer's nodes → the document, through commands |
| `updateDataModel` | the crate's tree |
| `deleteSurface` | removes the surface's nodes, through commands |
| `callRendererFunction` | the agent asks the renderer to do something local |
| `agentFunctionResponse` | the answer to one of those |

**The craftkit → agent direction** (`specification/v1_0/json/renderer_to_agent.json`)
is exactly four messages: `action`, `callAgentFunction`,
`rendererFunctionResponse`, `error`.

**The catalogue is not a message.** The protocol has **no** message that
carries a catalogue, and the plan does not invent one — that would break "a2ui
in, craftkit out" and the version rule (§9, R5). In a2ui the catalogue is the
**agent's** schema: the agent is given a catalogue and generates against it,
while the renderer publishes **capabilities**
(`specification/v1_0/docs/a2ui_protocol.md:93`, `:1612-1636`).

So:

- the **a2ui catalogue** — the projection of the **`CatalogEntry` registry**
  (`Ecosystem::catalog`) into a2ui's `catalog.json` shape — is published
  **inline**, in the door's capabilities, and **also written to a file**;
- the door publishes **capabilities**, in a2ui's shape, naming the catalogue
  it serves.

**And the first cut does both, because the protocol has two routes and an
agent may use either.**

| the route | how | who it needs |
|---|---|---|
| **inline** | the door's `a2uiRendererCapabilities` carries an **`inlineCatalogs`** array, each entry a `catalog_definition` (a2ui_protocol.md:1636) | the **agent** must say **`acceptsInlineCatalogs: true`** (`agent_capabilities.json` — the default is **false**) |
| **the file** | the door writes the catalogue to a **stated path** and writes **that path to `stderr`** — **never to stdout**, which carries the JSON-RPC stream and would be corrupted by a line of text | a host that can read a file |

**And an agent that accepts neither is refused**, with the reason: the door
has no third route, and the protocol has no message that carries a catalogue
(§9, R5).

**The capabilities are metadata, not a message.** a2ui exchanges them in
**transport metadata or a transport handshake**, and the protocol has no
message that carries them
(`specification/v1_0/docs/a2ui_protocol.md:93`, `:1612-1636`).

**And the channel is a handshake, because JSON-RPC has no metadata field.**
The door is a **JSON-RPC** door, and JSON-RPC 2.0 has nowhere to put
"transport metadata" — so the transport's own handshake is where the
capabilities go. a2ui's text gives the precedent by name: *"exchanged via
metadata or transport-specific handshakes (like Agent Cards in A2A or
**initialization in MCP**)"* (`:93`).

So the door's first request is **`initialize`**, and it is a **two-way**
handshake:

- the **request** carries the **agent's** capabilities — the door has to know
  what the agent can do **before** it can answer, because `inlineCatalogs`
  may only be provided when the agent has declared `acceptsInlineCatalogs:
  true` (`renderer_capabilities.json`'s `inlineCatalogs` description; the
  agent's default is **false**). This is a2ui's own precedent: the handshake
  is *"initialization in MCP"* (`a2ui_protocol.md:93`), and MCP's
  `initialize` carries the **client's** capabilities in the **request** for
  exactly this reason.
- the **result** carries the **`a2uiRendererCapabilities`** — the renderer's
  capabilities, in a2ui's own shape (`renderer_capabilities.json`), with the
  inline catalogue **only if** the request said the agent accepts them.

**And that is the only order that works.** The door cannot publish an inline
catalogue it does not know the agent will accept, and it cannot wait for the
agent's answer to send a result it has already begun to write — so the
capabilities go **up** in the request and **down** in the result.

**And the method names are a2ui's.** A JSON-RPC method is a string, and the
door uses the **a2ui message name** as the method name — `createSurface`,
`updateComponents`, `updateDataModel`, `deleteSurface`,
`callRendererFunction`, `agentFunctionResponse`, `action`, `callAgentFunction`,
`rendererFunctionResponse` — so a reader of the wire sees a2ui's vocabulary
and not a second one. `initialize` is the one name that is **not** a2ui's,
because the handshake is the transport's.

**And `supportedCatalogIds` names a catalogue, not a component.** The
renderer's field is *"the string identifiers of supported component and
function catalogs"* (`:1635`) — so the ids are **catalogues**, and a2ui's own
is `https://a2ui.org/specification/v1_0/catalogs/basic/catalog.json`. The
**component** ids are a different namespace, and they live inside the
catalogue.

So the door advertises **one** catalogue id — craftkit's own, minted with a
domain prefix the way a2ui recommends (*"It is recommended to prefix this
with an internet domain that you own"*, `agent_to_renderer.json:32`) — and:

- the **catalogue file** carries that id as its own `catalogId`;
- the door's capabilities name **that id**, and nothing else.

**And the surface's `catalogId` is the *agent's* choice, not the door's.**
`createSurface` is a message the **agent sends**
(`specification/v1_0/json/agent_to_renderer.json:30-32`), and its
`catalogId` is *"a string that uniquely identifies the default catalog for
this surface"*. So the door does not set it: the door **advertises** the
catalogue it serves and **serves the file** under that id, and the agent's
`createSurface` **names** one. **A surface that names a catalogue the door does
not serve is refused** — the house rule, applied to a catalogue the way it is
applied to a command id.

**And `catalogId` is optional** — `createSurface` requires only `surfaceId`
(`agent_to_renderer.json:56`). So a surface that **omits** it gets **the
door's own catalogue**, which is the only one the door serves; and a surface
that names a different one is refused. "Nothing" is an option, not a zero:
the omitted case is not an error, and the *wrong* case is.

So there are **two** places the id appears — the capabilities and the file's
own `catalogId` — and they are the same string; and a **third** place where it
*may* appear, the surface, which the agent fills in and the door checks.

**And the inline route is not an alternative to the file — it is the other
half of the first cut.** a2ui lets a renderer publish a catalogue **inline**,
in its capabilities (`inlineCatalogs`, `a2ui_protocol.md:1636`), which the
agent accepts when it says `acceptsInlineCatalogs` (`:1624`, and the default
is **false**). So the first cut does **both** — the inline route for an agent
that says yes, the file for one that does not, and the file for a person who
wants to read it. The projection is the same either way.

---

## 7. The arcs

The house's arc shape is the **port manifest** row
(`xtask/src/ports.rs:15-24`), the **claim row**, and a **README section**.
The manifest's own rows take a document in this tree as their `from` — for
example `from: "docs/survey-cost-and-meta.md"` — so these arcs fit it, with
`from: "docs/a2ui-compat-layer.md"`: **this plan** is the document the arcs
come from, not the survey it supersedes.

**So each arc is three edits and no more**: a row in `PORTS`, a row in the
README's **claim table**, and a section in the README. The claim table is an
**index**, and `cargo xtask lists` checks it against the port sections — every
port's `section` must have a claim row, and a claim whose truth starts with
**"built"** may not name a command the tree does not have
(`xtask/src/lists.rs:1011-1024`, `xtask/src/claims.rs:166-190`). A plan that
wrote the port row and forgot the claim row would fail the gate.

**Except for the arcs that add a crate — and those are three more.** Arcs
**F**, **G** and **I** add `craftkit-a2ui`, `craftkit-plugin-a2ui` and
`apps/craftkit-a2ui-server`. The gate refuses a crate that is not in the layering
table at all (`crates/craftkit-gates/src/lib.rs:437-443`), so an arc that
added a crate without those three edits would fail the build.

**And the row lands with the test.** A port row fails the gate until the
file it names exists (`xtask/src/ports.rs:2380-2386`), so each row's test
file is written **in the same arc** that adds the row — and the rows name
the files that arc will create, not files that exist today.

**And the whole shape, arc by arc:**

| arc | port rows | **new** crate? | section |
|---|---|---|---|
| **A** | 3 | — it **changes** the SDK and the MCP door | §1106 |
| **B** | 2 | — it **changes** the SDK | §1107 |
| **C** | 1 | — it **changes** the SDK | §1108 |
| **D** | 1 | — it **changes** the derive crate | §1109 |
| **E** | 2 | — it **changes** the SDK and the vector plugin | §1110 |
| **F** | 5 | **three** edits — `Cargo.toml`, the layer table, `docs/architecture.md` | §1111 |
| **G** | 2 | **three** | §1112 |
| **H** | 1 | — it **changes** the plugin | §1113 |
| **I** | 2 | **three** | §1114 |

**Nineteen rows, nine arcs.** An arc that lands in **two** crates has **two**
rows — a row is one crate — and an arc that lands in five has five. And
three arcs **add a crate**, which is three edits of their own on top of the
arc's three: the `Cargo.toml`, the row in the layer table
(`crates/craftkit-gates/src/lib.rs:104`), and the row in
`docs/architecture.md` — which `cargo xtask lists` checks against the gate's
table (`xtask/src/lists.rs:447-454`).

**These arcs are not ports from a family tree** — nothing is being ported
from another application. They are new work, and the manifest row is the
closest instrument the tree has. That is said here rather than left implied.

| arc | rule | from | to | module | command | test | section |
|---|---|---|---|---|---|---|---|
| **A** | the schema an agent is told is the same schema every door is told | `docs/a2ui-compat-layer.md` | `craftkit-sdk` | `src/schema.rs` | `craftkit-sdk::schema::to_json_schema` | `crates/craftkit-sdk/tests/schema.rs` | §1106 |
| **A** | the MCP door is told the same schema | `docs/a2ui-compat-layer.md` | `craftkit-mcp` | `src/lib.rs` | `craftkit-mcp::Server::tools` | `crates/craftkit-mcp/tests/tools.rs` — **added** by this arc | §1106 |
| **A** | one **JSON Schema** generator, and the doors that publish a schema publish it | `docs/a2ui-compat-layer.md` | `craftkit-sdk` | `src/ecosystem.rs` | `craftkit-sdk::Projection::json_schema` | `crates/craftkit-sdk/tests/schema.rs` | §1106 |
| **B** | a catalogue entry is a contribution | `docs/a2ui-compat-layer.md` | `craftkit-sdk` | `src/contribution.rs` | `craftkit-sdk::CatalogEntry` | `crates/craftkit-sdk/tests/catalog.rs` | §1107 |
| **B** | the catalogue entries are a view of the registry | `docs/a2ui-compat-layer.md` | `craftkit-sdk` | `src/ecosystem.rs` | `craftkit-sdk::Projection::entries` | `crates/craftkit-sdk/tests/catalog.rs` | §1107 |
| **C** | the **vocabulary** view is a view of the registry | `docs/a2ui-compat-layer.md` | `craftkit-sdk` | `src/ecosystem.rs` | `craftkit-sdk::Projection::catalog` | `crates/craftkit-sdk/tests/catalog.rs` | §1108 |
| **D** | the compiler writes the catalogue entry | `docs/a2ui-compat-layer.md` | `craftkit-derive` | `src/a2ui_component.rs` | `craftkit-derive::A2uiComponent` | `crates/craftkit-derive/tests/a2ui.rs` | §1109 |
| **E** | the SVG path data is read in one place | `docs/a2ui-compat-layer.md` | `craftkit-sdk` | `src/svgpath.rs` | `craftkit-sdk::svgpath` | `crates/craftkit-sdk/tests/svgpath.rs` | §1110 |
| **E** | the vector plugin reads its path data from the SDK | `docs/a2ui-compat-layer.md` | `craftkit-plugin-vector` | `src/svgimport.rs` | `vector.import.svg` | `crates/craftkit-plugin-vector/tests/svgimport.rs` | §1110 |
| **F** | a2ui's shapes are craftkit's shapes | `docs/a2ui-compat-layer.md` | `craftkit-a2ui` | `src/model.rs` | `craftkit-a2ui::model` | `crates/craftkit-a2ui/tests/model.rs` | §1111 |
| **F** | the a2ui catalogue is a projection of the entries | `docs/a2ui-compat-layer.md` | `craftkit-a2ui` | `src/catalog.rs` | `craftkit-a2ui::catalog` | `crates/craftkit-a2ui/tests/catalog.rs` | §1111 |
| **F** | the function table names what the layer does and what the agent does | `docs/a2ui-compat-layer.md` | `craftkit-a2ui` | `src/functions.rs` | `craftkit-a2ui::functions` | `crates/craftkit-a2ui/tests/functions.rs` | §1111 |
| **F** | the property-name table is the layer's | `docs/a2ui-compat-layer.md` | `craftkit-a2ui` | `src/defs.rs` | `craftkit-a2ui::defs` | `crates/craftkit-a2ui/tests/defs.rs` | §1111 |
| **F** | the re-derived shapes are the layer's | `docs/a2ui-compat-layer.md` | `craftkit-a2ui` | `src/schema.rs` | `craftkit-a2ui::schema` | `crates/craftkit-a2ui/tests/schema.rs` | §1111 |
| **G** | the components paint themselves | `docs/a2ui-compat-layer.md` | `craftkit-plugin-a2ui` | `src/components.rs` | `craftkit-plugin-a2ui::components` | `crates/craftkit-plugin-a2ui/tests/components.rs` | §1112 |
| **G** | the `a2ui` layer is declared by the plugin | `docs/a2ui-compat-layer.md` | `craftkit-plugin-a2ui` | `src/lib.rs` | `craftkit-plugin-a2ui::components` | `crates/craftkit-plugin-a2ui/tests/components.rs` | §1112 |
| **H** | a surface is a command | `docs/a2ui-compat-layer.md` | `craftkit-plugin-a2ui` | `src/commands.rs` | `a2ui.surface.apply` | `crates/craftkit-plugin-a2ui/tests/commands.rs` | §1113 |
| **I** | the door speaks a2ui | `docs/a2ui-compat-layer.md` | `apps/craftkit-a2ui-server` | `src/main.rs` | `a2ui.surface.apply` | `apps/craftkit-a2ui-server/tests/door.rs` | §1114 |
| **I** | the door renders **headlessly** and maps a click to a command | `docs/a2ui-compat-layer.md` | `apps/craftkit-a2ui-server` | `src/render.rs` | `a2ui.component.invoke` | `apps/craftkit-a2ui-server/tests/render.rs` | §1114 |

**The `module` field is crate-relative.** The gate joins it to the crate's
directory (`xtask/src/ports.rs:2388-2392`), so it is `src/schema.rs` and not
`crates/craftkit-sdk/src/schema.rs`.

**Arc I is an app, and it says so.** `to` is joined to the tree's root as
well as to `crates/` (`xtask/src/ports.rs:2388-2392`), so `apps/craftkit-a2ui-server`
is a legal target — and **the door carries no command of its own**: every
command it runs is the plugin's. The row therefore names the command that
carries it, `a2ui.surface.apply`, and the door's own identity is its
**test** (`apps/craftkit-a2ui-server/tests/door.rs`).

**And the two new crates are named in the gate, not only in the layer table.**

`craftkit-plugin-a2ui` is a **plugin**, and **not** one of the nine
**batteries** — the gate's `BATTERIES` are *"the nine the README counts"*
(`crates/craftkit-gates/src/lib.rs:307`), and the other plugins (the picture,
the shape, the typed value, the CLAP and the VST3 door) are **plugins**, whose
commands are a struct with `#[derive(Command)]` (`:300-306`). So the compat
plugin's **commands** are `#[derive(Command)]` structs, and its
**components** are the `components!` table (§2.1) — and the gate's
`BATTERIES` list does **not** change.

**Two things the index does anyway.** `xtask/src/index.rs:89-104`
auto-discovers every `craftkit-plugin-*` with a `src/` directory, so `a2ui`
is found as a battery name whatever the gate's list says; and
`xtask/src/lists.rs:859-877` then reads `craftkit-plugin-a2ui/src/lib.rs` and
compares **any command count it states** with the index's count. So the
plugin's `lib.rs` either states no count, or states the right one.

And `apps/craftkit-a2ui-server` is a **front end**, so
**`craftkit-a2ui-server`** — the package name, as `FRONT_ENDS` holds them
(`:274` holds `"craftkit-window"`) — is added to **`FRONT_ENDS`**, and the
gate holds the door to the
thin-front-end rule: it may read the kit's state and act through the
registry, and it may not write the document, call the evaluator, or keep a
`Plan`.

**And one edit has no port row, because it is to the gate and not to a
crate**: the `FRONT_ENDS` addition. A port row names a crate and a file
(`xtask/src/ports.rs:15-24`), and the gate's list is neither — so it is
named here instead, and the claim row for the arc says it.

**Arc A is the prerequisite for everything.** It moves the `Doc` → JSON Schema
mapping out of `craftkit-mcp` (L3) and into the SDK (L2), so the MCP door, the
bridge and craftkit's own doors are **one** schema. This is where "one source"
stops being a principle and becomes code.

**And "the bridge" is not free.** The bridge builds its tools from
`projection.schema(entry.id)` (`apps/craftkit-bridge/src/lib.rs:183-201`),
and that is a **descriptor** — `id`, `label`, `area`, `capability`, `doc`,
`parameters[{name, kind, required, help}]`
(`crates/craftkit-sdk/src/ecosystem.rs:525-558`) — **not** JSON Schema. So
Arc A's third row adds **`Projection::json_schema(id)`** — the **moved**
mapping, over the JSON Schema — beside the **descriptor** that
`Projection::schema(id)` already returns
(`crates/craftkit-sdk/src/ecosystem.rs:525-558`).

**And the descriptor survives, and that is the point.** The bridge reads
`id` and `doc` out of what it is given — `schema.get("id")`,
`schema.get("doc")` (`apps/craftkit-bridge/src/lib.rs:190-191`, `:261-263`) —
so a schema that replaced the descriptor would leave the bridge's tools
nameless. **So there are two things, not one**: the **descriptor**, which is
what the bridge publishes and always did, and the **JSON Schema**, which is
what the MCP door publishes. The **moved** mapping is the second, and the
first is **not** re-expressed.

**And the bridge has no port row, and needs none**, because **the bridge's
own code does not change**: it asks for the descriptor and still gets one.

**And "one schema for every door" is therefore restated.** It is **one JSON
Schema generator**, and every door that publishes a **schema** publishes
that one. The bridge publishes a **descriptor**, which is a different thing.

**And O1's acceptance test is **deferred**.** It is *"a test asserting that
**both doors' `tools/list` are generated from the one `Projection::catalog()`"*
(`survey-a2ui.md:495-506`) — and these arcs change the **SDK side**, not the
two doors. So that test is O1's own remaining half, and the plan says so
rather than claiming it (R13).

**Arc A is not O1.** O1, as the survey writes it, is `Projection::catalog()`
— the **vocabulary view** — and it names the MCP door the first **consumer of
the catalogue**, whose `schema_of` the catalogue *generalises* rather than
replaces. The **move** of the mapping itself is the plan's own work, and the
survey's O1 cost row does not price it.

**And Arc C is the SDK half of O1 — not all of it.** O1's acceptance test is
*"a test asserting that **both doors' `tools/list` are generated from the one
`Projection::catalog()`"* (`survey-a2ui.md:495-506`): the MCP door builds its
tools from `session.registry().all()` (`crates/craftkit-mcp/src/lib.rs:142-154`)
and the bridge from `projection.menu(&["command", "effect", "tool"])`
(`apps/craftkit-bridge/src/lib.rs:183-201`). **These arcs change the SDK
side; they do not change the two doors.** So the two-doors test is
**deferred** — it is O1's own acceptance criterion, and it is not this
plan's work. The plan delivers the **one** `Projection::catalog()`; the
doors' use of it is O1's remaining half, and O2 then prunes each door on
purpose.

**Arc H is the first arc with a command of its own**, because §5 insists that
writes are commands — and craftkit has **no** generic "add a node of this
kind" command (the only `core.*` commands are `core.appearance.*` and
`core.layer.*`, `crates/craftkit-sdk/src/appearance.rs:61` and
`layer.rs:66`). The compat layer needs its own:

```text
a2ui.surface.create    a2ui.surface.apply    a2ui.surface.delete
a2ui.data.set         a2ui.component.invoke
```

### 7.1 The order, and what each arc ships

| arc | ships | cost |
|---|---|---|
| A | one schema for every door | small — a move, and a golden test that MCP's output did not change |
| B | `CatalogEntry` and its registry | small, but **five SDK edits** (§3.1) |
| C | `Projection::catalog()` | small |
| D | `#[derive(A2uiComponent)]` and `components! { … }` | **medium** — the first new macro |
| E | **the SVG path data is read in one place** — the parser moves to L2 and gains an **arc** | **medium** — a move, a conversion, and two crates' worth of edits (§4.3) |
| F | the a2ui crate: the protocol, the data model, the bindings, the box layout, **the a2ui catalogue projection** | medium–large |
| G | eleven components, eight of them with work | medium |
| H | the commands the door runs | small |
| I | the door | small |

**Arc E is not a2ui's work, and it is in the list anyway.** The vector
plugin's SVG path data reader is the one the layer cannot reach — it is in
**L4**, and the **CAD battery has its own reader**, which this plan does not
touch — and it **refuses arcs**, which a2ui's `svgPath` may carry. So the
**reader** moves to **L2** (the SDK), the vector plugin's importer stays and
calls it from there, and it gains an **arc-to-cubic** conversion (§4.3). It is
an arc of its own because it changes two crates and neither of them is a2ui's.

**Nothing ships before Arc I.** Arcs A–C are invisible refactors;
Arc D is a macro with no users yet; E is a move; F and G are a library with no
door. That is worth saying plainly: **Arc I is the first thing that does
anything**, and everything before it is preparation.

**And Arc I is **headless for a human**.** The door speaks a2ui and serves
its surface on a **control channel** (§5.3); it opens no window, and a window
that attaches to somebody else's session is not in the tree. So the plan
delivers **a surface an agent can drive and a control-channel client can
see** — which is the useful thing — and it does not deliver a window.

---

## 8. The tests

The house rule: **the crate's own rule has a test.** A rule with no test is a
habit.

| arc | the test that must exist |
|---|---|
| A | MCP's `tools/list` output is **unchanged** — a golden test, or the existing ones break |
| B | a `CatalogEntry` with a malformed id is refused, like every other contribution; a `CatalogEntry` in a layer that is not linked in is refused |
| C | `Projection::catalog()` lists every registered **command, tool, panel and kind** and nothing else; a registered one cannot be missing; nothing is listed twice |
| D | the derive writes the struct, the reader, the `ParamDoc`s and the two statics; a duplicate prop is refused; `Option<T>` is optional and `T` is not; `Either` needs `or`; the row's key makes the struct and the statics. **Arc D's test is token-level**: it expands a row and checks the **emitted tokens** — the struct, the reader, the two paths, the statics — and compiles **nothing** against the layer, because the layer does not exist yet when the derive lands |
| E | the SVG path data reader reads every command SVG writes, **including the arc**; the vector plugin's importer still reads a file it read before — a golden test, or the plugin's own tests break |
| F | a surface round-trips: the same tree in gives the same nodes out; an unknown component is **refused**, not faked; a binding to a missing path is refused; **the catalogue carries one `catalogId`**, and its `functions` map holds the five local functions and the declared agent-side ones, each with its `allowedCallers`; **the schema text's property set is the table's** — the layer's two artefacts cannot drift, and this is **Arc F's** test because the layer is where both live |
| G | each component paints something, and the picture is not empty; **the published schema's property set, minus the exempt entries, is the row's** — the exempt entries are `component`, which the compiler stamps, and `@checks`, which the layer removes — and **each prop's published `$ref` is the definition the row declared** — so `#[a2ui_prop(DynamicString)]` is checked, not decorative; the compile-time assertion against the layer's table holds; and **the catalogue lists exactly the registered components**, in a2ui's own names, with each one's schema the published one — all of it checked **here**, because this is where the expansion is used and the layer exists |
| H | a surface apply is **undoable** and is rolled back when it fails; an unknown command is refused |
| I | the door answers `createSurface` with a surface, and a button runs its command; a function name that is in neither the table nor the catalogue's `functions` is **refused**, and a surface naming a catalogue the door does not serve is **refused**; **the door's `Render` draws a picture with no window** — the rasteriser is software — and **a click on an `a2ui.*` node runs `a2ui.component.invoke`** |

**The refusals are the tests that matter.** "Unknown component is refused" and
"unknown command is refused" are the two places where craftkit's rules and
a2ui's placeholders disagree, and the house rule wins.

**And the two catalogues are tested separately.** Arc C's test is about the
**vocabulary view** — commands, tools, panels, kinds — and says nothing about
components. The **a2ui catalogue** is a projection of the **`CatalogEntry`
registry** into a2ui's `catalog.json` shape, and that projection lives in
`craftkit-a2ui` — so it is **Arc F's** test. Arc **D**'s test is that the
compiler writes the **entry** right: the resolved props are craftkit's, and
the entry's `name` is the a2ui name and its `id` is craftkit's. The
**published** half of that — the schema in a2ui's shape — is **Arc G's**
test, because that is where the expansion is used and the layer's shapes
are reachable.

---

## 9. The risks

| # | the risk | what it would cost | the answer |
|---|---|---|---|
| R1 | **moving the schema mapping changes MCP's output** | the agent door's contract breaks | a golden test on `tools/list` **before** the move (Arc A) |
| R2 | the derive is the first new macro in the tree | a bad macro is a bad door | the derive is small, and it **composes** `expand_node_kind` rather than reimplementing it |
| R3 | **the box layout is in `craftkit-a2ui` (L3), not in the plugin** | a second copy, later, when another renderer needs it | the promotion trigger is written down (§4.1); L3 is chosen because a plugin may not depend on another plugin |
| R4 | the data model becomes a second source of truth | two truths | the tree is a view; writes are commands (§5) |
| R5 | a2ui's protocol is **versioned** (`v0_8` … `v1_0`) | the door speaks a version the tree does not | the door accepts **one** version and **refuses** the others — craftkit's rule, not a2ui's adapter chain |
| R6 | the door is a new executable | a new thing to keep | it is an app, like `craftkit-mcp-server`; exempt from the layering rule, and nothing else changes |
| R7 | the components' painters are wrong | a broken picture | each painter has a test that the picture is not empty (Arc G) |
| R8 | **the SDK changes** — a new contribution type and a moved mapping | the SDK is depended on by everything; a mistake is everywhere | the whole tree is in scope for `cargo xtask ci`; Arc A carries a golden test |
| R9 | the `a2ui` layer is not linked in | the ecosystem refuses to start | the layer is declared by the plugin (§3.1), and `check_layers` says so at start-up rather than at draw time |
| R10 | the whole arc is a **feature** the tree does not need | a lot of work for nothing | **this is the open question of §6.3 of the survey** — and it is a question for the owner, not for the tree |
| R11 | **the SVG path reader must move to L2, and must learn arcs** | the reader moves to the SDK, and the SDK grows | the reader is promoted rather than copied (§4.3); the vector plugin's importer stays and calls it; an arc it cannot convert is **refused** with the reason, and the catalogue says which commands the reader accepts |
| R13 | **O1's two-doors test is deferred** — the plan delivers the one `Projection::catalog()`, not the two doors' use of it | O1's acceptance criterion is not met by these arcs | it is **stated**, not hidden: §7 says the plan is the **SDK half** of O1, and the doors' half is O1's own remaining work (`survey-a2ui.md:495-506`) |
| R12 | the **published** schema is a2ui's, and the craftkit props are the resolved ones — two schemas | a component that is described twice can disagree with itself | **one row writes both** — the derive writes the names and the call, the layer owns the shapes — and the tests say so (Arcs **D**, **F** and **G**) |

---

## 10. What it does not do

- It does **not** expose craftkit's **other** kinds as a2ui components — only
  the eleven a2ui ones are, and the other eighty-eight are not. Exposing them
  would be a second catalogue, and the one a2ui's negotiation exists to
  reconcile. An application that wants `layout.spread` addressable declares
  that itself.
- It does **not** let a model choose craftkit's kinds. a2ui's model chooses
  *widgets*; craftkit's model chooses *commands*. The compat layer lets an
  a2ui-speaking agent **drive** craftkit, and that is all.
- It does **not** add a renderer. craftkit has one seam and two canvases; the
  layer uses the seam.
- It does **not** touch the rasteriser, the camera, the font metrics or the
  text layout.
- It does **not** add a way to run a command. Every **command** the layer
  runs is `Session::execute`.
- It does **not** write a `catalog.json` by hand, and it does **not** invent
  a protocol message.
- It does **not** fetch anything over a network (§4.2).

---

## 11. The first arc, in full

So the plan can be started without further reading:

**Arc A — one schema for every door.**

1. Move the `Doc` → JSON Schema mapping from `crates/craftkit-mcp/src/lib.rs:439-488`
   into the SDK — a new `crates/craftkit-sdk/src/schema.rs`, beside
   `Projection::schema` (`crates/craftkit-sdk/src/ecosystem.rs:525`).
2. `craftkit-mcp` calls the SDK's function. **Its output must not change.**
3. A golden test on the MCP `tools/list`, taken **before** the move.
4. `cargo test -p craftkit-mcp -p craftkit-sdk -p craftkit-gates`, then
   `cargo xtask ci` — the SDK is depended on by everything, so the whole tree
   is in scope. **And that run is heavy**: from a cold cache it is about
   **twenty-three minutes** on this machine, and this is a **shared** one. So
   it is run **once, at the end of the batch**, not after every arc — and
   `cargo xtask ci` says which packages it checked and **which it left out**,
   so a reader knows what was not covered.

**That is the whole first arc**, and it is worth doing on its own: it is a
prerequisite, it is small, it is invisible, and it makes every later arc
cheaper.

---

## 12. The kind count, and how to check it

This plan says craftkit has **99 node kinds**. The count is checkable, and the
method matters more than the number — a literal-attribute grep alone
**undercounts by 23**:

> **distinct kind names in the shipped crates**: every `#[node_kind(…)]`
> attribute under `crates/` — **76** distinct, because `geometry.rect` is
> declared three times (twice in the derive crate's doc and test, once for
> real) — **plus** every invocation of the `art_kind!`, `track_kind!`,
> `entity_kind!`, `item_kind!` and `block_kind!` macro shims, which expand to
> the same attribute and are **23** more. **76 + 23 = 99.**
>
> One further kind exists only in an integration test (`test.panic.kind`,
> `apps/craftkit-studio/tests/guard.rs:14`) and is not shipped.

The shims are real node kinds: `art_kind!`
(`crates/craftkit-plugin-art/src/lib.rs:92`) expands to
`#[derive(NodeKind)] #[node_kind(kind = $kind, label = $label, layer = "art", evaluates)]`.
