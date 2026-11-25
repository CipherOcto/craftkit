# The effectcraft plan: what the upstream has and the fx battery does not

This plan is about the upstream — a motion-graphics engine whose commands are
the reference for this tree's **fx battery**. It names the gaps between what
that upstream offers and what `craftkit-plugin-fx` holds today, and it turns
the four gaps the survey called **PORT** into **six arcs**. Those arcs reserve
**README sections §1208 to §1213**, one section each, in the order they are
written here.

## 0. The rule

A capability the upstream has and the fx battery does not is ported by
**adding a command to the registry**, and every part of it — the in and out
points, what is done to them, the mask's feather, the mask's expansion, the
mask's opacity, the keyframe's tangents, the expression's errors — is
reachable through the registry and through nothing else.

## 1. What the survey found

| capability | upstream path:line | craftkit's state | verdict |
|---|---|---|---|
| Layer timing: in and out points, time stretch, time reverse, freeze frame | `crates/engine/src/cmd/layer_time.rs:247`, `:265`, `:270`, `:272` | absent — `fx.time.*` is the composition's time only, and a layer carries no in or out point | PORT |
| Mask feather, mask expansion and mask opacity | `crates/project/src/build.rs:86`, `crates/script/src/matchnames.rs:55` | partial — the mask node keeps `feather`, `opacity` and `expansion` but no command sets them | PORT |
| Writing a keyframe's spatial tangents down | `crates/engine/src/cmd/anim_tools.rs:393`, `crates/engine/src/cmd/key_labels.rs` | partial — every spatial path has auto tangents and nothing writes them | PORT |
| The errors an expression has, as a report a user can read | `crates/engine/src/cmd/expr_tools.rs:400` | absent — `fx.prop.expression` sets an expression but nothing reports a bad one | PORT |
| A catalogue of hundreds of effects, each with its parameters, kinds and defaults | `crates/effects/src/catalog.rs` | partial — the battery's catalogue is six effects on purpose | LATER |
| Layer styles: drop shadow, glow, bevel and emboss, satin, stroke, overlays, with a global light | `crates/engine/src/cmd/styles.rs:237`, `:248` | absent — the battery has effects on a layer and a set of layer switches, but no style group | LATER |
| 3D layers, cameras and lights | `crates/engine/src/cmd/three_d.rs:721`, `:712`, `crates/engine/src/cmd/camera_cmds.rs:44` | absent — the battery has no 3D kind | LATER |
| Shape layers: paths, trim paths, repeaters, round corners, offset, gradient strokes | `crates/engine/src/cmd/paths.rs` | absent — the battery holds no shape layer | LATER |
| Text layers and text animators with range selectors | `crates/engine/src/cmd/text_anim.rs:455`, `:473` | absent — the battery has no text layer | LATER |
| A track matte: one layer used as the matte of another | `crates/engine/src/cmd/layer.rs` | absent — the battery has parenting and layer switches, but no matte relation | LATER |
| Expressions with a real script language (`wiggle`, `valueAtTime`, `thisComp`, `posterizeTime`) | `crates/expr/src/`, `crates/engine/src/cmd/anim_tools.rs:393` | partial — the battery's expression is a small arithmetic language of its own and refuses the upstream's names | LATER |
| Lottie import and export | `crates/engine/src/cmd/lottie.rs:58`, `crates/lottie/src/` | absent | LATER |
| Roto brush and segmentation, and object matte | `crates/engine/src/cmd/roto_cmds.rs:43`, `:314` | absent | LATER |
| Motion tracking and stabilisation | `crates/engine/src/cmd/track.rs:23` | absent | LATER |
| Footage, image sequences and audio in a composition | `crates/engine/src/cmd/file.rs`, `crates/media/src/` | absent — the battery holds layers as property trees with no media kind | SKIP |
| Render queue, render settings, output modules and the export encoders | `crates/engine/src/cmd/render_queue.rs:1061`, `:1072`, `:1081` | absent | SKIP |

The survey read the upstream's index documents and command registries and
compared them against the fx battery, and it read the battery's own code. Its
finding is that four capabilities are genuinely missing and worth porting, and
that the rest are either out of scope for this battery or are larger than a
single arc.

The four missing ones are the **layer's timing**, the **mask's own
parameters**, the **keyframe's spatial tangents** and the **expression's
errors**. Two of them are close to the tree already: the mask node keeps
`feather`, `opacity` and `expansion`, and every spatial path already has auto
tangents — but no command writes either, so what exists is a field nobody can
set. The other two are wholly absent.

The **LATER** rows — the wider effect catalogue, layer styles, 3D layers,
shape layers, text layers and animators, track mattes, a real script language,
Lottie import and export, roto and motion tracking — are **later work**. They
are named in §5 with the reason each waits. The **SKIP** rows — media, and
the render queue — are out of scope for this battery and are not arcs of this
plan at all.

## 2. The arcs

Each arc is three edits and no more: a row in the port manifest, a row in the
README's claim table, and a section in the README. The README sections this
plan reserves are **§1208, §1209, §1210, §1211, §1212 and §1213** — six
sections, one per arc, in the order below, with no gap and no repeat. The
port manifest's rows take this document as their `from`, because this plan is
the document the arcs come from.

| arc | rule | from | to | module | command | test | section |
|---|---|---|---|---|---|---|---|
| **A** | a layer carries an in point and an out point | `docs/plans/effectcraft.md` | `craftkit-plugin-fx` | `src/layer_time.rs` | `fx.layer.inout` | `crates/craftkit-plugin-fx/tests/layer_time.rs` | §1208 |
| **B** | a layer's in and out points are stretched, reversed or frozen | `docs/plans/effectcraft.md` | `craftkit-plugin-fx` | `src/time_ops.rs` | `fx.layer.time.stretch`, `fx.layer.time.reverse`, `fx.layer.time.freeze` | `crates/craftkit-plugin-fx/tests/layer_time.rs` | §1209 |
| **C** | a mask's feather and its expansion are written down | `docs/plans/effectcraft.md` | `craftkit-plugin-fx` | `src/masks.rs` | `fx.mask.feather`, `fx.mask.expansion` | `crates/craftkit-plugin-fx/tests/masks.rs` | §1210 |
| **D** | a mask's opacity is written down | `docs/plans/effectcraft.md` | `craftkit-plugin-fx` | `src/masks.rs` | `fx.mask.opacity` | `crates/craftkit-plugin-fx/tests/masks.rs` | §1211 |
| **E** | a keyframe's spatial tangents are written down | `docs/plans/effectcraft.md` | `craftkit-plugin-fx` | `src/spatial.rs` | `fx.keyframe.tangent` | `crates/craftkit-plugin-fx/tests/spatial.rs` | §1212 |
| **F** | an expression's errors are a report a user can read | `docs/plans/effectcraft.md` | `craftkit-plugin-fx` | `src/expressions.rs` | `fx.prop.expression.report` | `crates/craftkit-plugin-fx/tests/expressions.rs` | §1213 |

The survey's four **PORT** rows become six arcs because two of them are two
things. The layer-timing row is a row and a set of operations on that row:
**A** adds the in and out points themselves, and **B** adds what is done to
them — stretch, reverse, freeze. The mask row is the same shape: **C** adds
the feather and the expansion, and **D** adds the opacity. Splitting them
keeps each arc to one rule and one claim row, which is what the claim table
can be checked against. The other two rows — the tangents and the expression's
errors — are one rule each and stay one arc.

## 3. The order, and what each arc ships

| arc | ships | cost |
|---|---|---|
| **A** | the layer's in point and out point, and the commands that set and read them | small — two fields on the layer, and the composition's own time is already there to bound them |
| **B** | stretch, reverse and freeze on those points | medium — three operations over the layer's time, and each one changes what the layer evaluates to |
| **C** | the mask's feather and its expansion | small — the mask node already keeps both fields, and only the commands were missing |
| **D** | the mask's opacity | small — the third field the mask node already keeps, and the same shape as C |
| **E** | a keyframe's spatial tangents, written down | medium — the auto tangents have to become a default that a command may override |
| **F** | the report of an expression's errors | medium — the errors have to be collected and read, not only refused |

**A comes first because B has nothing to stand on**: stretch, reverse and
freeze are operations on an in point and an out point, and those arrive in
**A**. **C** and **D** follow together, because they are the same rule written
twice — the mask's own fields — and the fields are already there. **E** and
**F** are last: they change what the evaluator computes rather than what a
layer carries, and they are the two arcs whose rules the rest of the battery
depends on.

## 4. The tests

The house rule: the crate's own rule has a test.

| arc | the test that must exist |
|---|---|
| **A** | a layer's in point and out point round-trip through the commands; an out point before the in point is refused; a point outside the composition's time is refused |
| **B** | a stretch of less than or equal to zero is refused; a reversed layer reads back reversed; a frozen layer holds its frame; a layer with no in and out points is refused by the operations that need them |
| **C** | a mask's feather and expansion round-trip; a negative feather is refused; a mask with no feather keeps the field it had |
| **D** | a mask's opacity round-trips; an opacity outside the range is refused; a mask's opacity is independent of the layer's own opacity |
| **E** | a keyframe's written tangents are read back, and are used in place of the auto tangents; a tangent that does not describe a curve is refused |
| **F** | a bad expression yields a report naming the error and where it is; a good expression yields an empty report; the report is a value and never a panic |

## 5. What it does not do

The **LATER** rows are named here with the reason each waits, and none of them
is an arc of this plan.

- **A catalogue of hundreds of effects, each with its parameters, kinds and defaults** — the battery's catalogue is six effects on purpose, and widening it is a decision about the catalogue's size, not a port.
- **Layer styles: drop shadow, glow, bevel and emboss, satin, stroke, overlays, with a global light** — a style group is a new kind of thing on a layer, and nothing else in the battery carries one.
- **3D layers, cameras and lights** — the battery has no 3D kind, and a camera is a question about a camera, not about an effect.
- **Shape layers: paths, trim paths, repeaters, round corners, offset, gradient strokes** — the battery holds no shape layer; the paths belong to the vector battery.
- **Text layers and text animators with range selectors** — the battery has no text layer; the text belongs to the text battery.
- **A track matte: one layer used as the matte of another** — the battery has parenting and layer switches, but a matte is a third relation between layers, and it wants a rendering decision this plan does not make.
- **Expressions with a real script language** — the battery's expression is a small arithmetic language of its own and refuses the upstream's names on purpose; a script language is a different language, not a port of it.
- **Lottie import and export** — a file format is an interchange, and the upstream's format is its own.
- **Roto brush and segmentation, and object matte** — roto is an interactive painting surface, and a command is not a brush.
- **Motion tracking and stabilisation** — tracking is an analysis that runs before any command runs.

The **SKIP** rows are out of scope for this battery entirely.

- **Footage, image sequences and audio in a composition** — the battery holds layers as property trees with no media kind; media belongs to the film battery.
- **Render queue, render settings, output modules and the export encoders** — the render queue belongs to the film battery and the application, not to a battery of effects.

## 6. The open questions

The survey says what it could not read, and these are its own words, condensed.

The survey read the upstream's index documents and command registries, but not
every upstream crate; a capability living in a crate it never opened may
already be ported elsewhere in craftkit. The survey did not read craftkit's
other batteries — shapes, vector, text, film, pdf — so the skip verdicts that
name them are reasoned from their names, not from their code. Whether craftkit's
core, and not only the battery, already offers anything about page breaks, tab
stops, line numbering, track-changes switching, mask feather or layer timing
is **unsure**: the survey grepped the battery crate only. The upstream's
file-format and layout parity documents were not read, so a capability listed
only there is in neither table.

**What would settle them:** reading the upstream crates the survey did not
open, and reading the other batteries and the core, would turn each of these
into a fact — and would say whether an arc above duplicates something the tree
already has. Until then, the arcs above are planned on the survey's findings
as it states them, and an arc that turns out to exist already is dropped
rather than ported twice.
