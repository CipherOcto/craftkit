# The effectcraft plan: what the upstream has and the fx battery does not

This plan is about the upstream — a motion-graphics engine whose commands are
the reference for this tree's **fx battery**. It names the gaps between what
that upstream offers and what `craftkit-plugin-fx` holds today, and it turns
the four gaps the survey called **PORT** into **six arcs**, and — in a **second
wave** — every capability the upstream holds back as **later** or **skipped**,
into **thirteen more arcs**. The plan holds **19 arcs**, and they reserve
**README sections §1211 to §1326**, one section each, in the order they are
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
Lottie import and export, roto and motion tracking — are the **second wave**
of this plan's arcs (§2): the survey's verdict there was a judgement about
size, not about scope. The **SKIP** rows — media, and the render queue — are
arcs too, and they land in the **film** battery, because that is the battery
that owns them.

## 2. The arcs

Each arc is three edits and no more: a row in the port manifest, a row in the
README's claim table, and a section in the README. The README sections this
plan reserves are **§1211, §1212, §1213, §1214, §1215 and §1216** — six
sections, one per arc, in the order below, with no gap and no repeat — and the
second wave's **§1314 to §1326**, thirteen more, one per arc, in the order
they are written below. The port manifest's rows take this document as their
`from`, because this plan is the document the arcs come from.

| arc | rule | from | to | module | command | test | section |
|---|---|---|---|---|---|---|---|
| **A** | a layer carries an in point and an out point | `docs/plans/effectcraft.md` | `craftkit-plugin-fx` | `src/layer_time.rs` | `fx.layer.inout` | `crates/craftkit-plugin-fx/tests/layer_time.rs` | §1211 |
| **B** | a layer's in and out points are stretched, reversed or frozen | `docs/plans/effectcraft.md` | `craftkit-plugin-fx` | `src/time_ops.rs` | `fx.layer.time.stretch`, `fx.layer.time.reverse`, `fx.layer.time.freeze` | `crates/craftkit-plugin-fx/tests/layer_time.rs` | §1212 |
| **C** | a mask's feather and its expansion are written down | `docs/plans/effectcraft.md` | `craftkit-plugin-fx` | `src/masks.rs` | `fx.mask.feather`, `fx.mask.expansion` | `crates/craftkit-plugin-fx/tests/masks.rs` | §1213 |
| **D** | a mask's opacity is written down | `docs/plans/effectcraft.md` | `craftkit-plugin-fx` | `src/masks.rs` | `fx.mask.opacity` | `crates/craftkit-plugin-fx/tests/masks.rs` | §1214 |
| **E** | a keyframe's spatial tangents are written down | `docs/plans/effectcraft.md` | `craftkit-plugin-fx` | `src/spatial.rs` | `fx.keyframe.tangent` | `crates/craftkit-plugin-fx/tests/spatial.rs` | §1215 |
| **F** | an expression's errors are a report a user can read | `docs/plans/effectcraft.md` | `craftkit-plugin-fx` | `src/expressions.rs` | `fx.prop.expression.report` | `crates/craftkit-plugin-fx/tests/expressions.rs` | §1216 |

The survey's four **PORT** rows become six arcs because two of them are two
things. The layer-timing row is a row and a set of operations on that row:
**A** adds the in and out points themselves, and **B** adds what is done to
them — stretch, reverse, freeze. The mask row is the same shape: **C** adds
the feather and the expansion, and **D** adds the opacity. Splitting them
keeps each arc to one rule and one claim row, which is what the claim table
can be checked against. The other two rows — the tangents and the expression's
errors — are one rule each and stay one arc.

**The second wave: the whole upstream.**

| arc | rule | from | to | module | command | test | section |
|---|---|---|---|---|---|---|---|
| **G** | an effect is a catalogue entry with its parameters, kinds and defaults | `docs/plans/effectcraft.md` | `craftkit-plugin-fx` | `src/catalog.rs` | `fx.effect.catalog` | `crates/craftkit-plugin-fx/tests/catalog.rs` | §1314 |
| **H** | a layer carries a style group — drop shadow, glow, bevel and emboss, satin, stroke, overlays — and the composition carries the global light | `docs/plans/effectcraft.md` | `craftkit-plugin-fx` | `src/styles.rs` | `fx.layer.style`, `fx.style.light` | `crates/craftkit-plugin-fx/tests/styles.rs` | §1315 |
| **I** | a layer may be three-dimensional, and the composition may carry cameras and lights | `docs/plans/effectcraft.md` | `craftkit-plugin-fx` | `src/three_d.rs` | `fx.layer.three_d`, `fx.camera.create`, `fx.light.create` | `crates/craftkit-plugin-fx/tests/three_d.rs` | §1316 |
| **J** | the paths of a shape are read and written in the vector battery | `docs/plans/effectcraft.md` | `craftkit-plugin-vector` | `src/paths.rs` | `vector.path.trim`, `vector.path.repeat`, `vector.path.round`, `vector.path.offset`, `vector.stroke.gradient` | `crates/craftkit-plugin-vector/tests/paths.rs` | §1317 |
| **J** | a shape layer is a layer whose content is a path | `docs/plans/effectcraft.md` | `craftkit-plugin-fx` | `src/shapes.rs` | `fx.layer.shape` | `crates/craftkit-plugin-fx/tests/shapes.rs` | §1317 |
| **K** | a range selector is a range of a text run | `docs/plans/effectcraft.md` | `craftkit-plugin-text` | `src/selector.rs` | `text.range.selector` | `crates/craftkit-plugin-text/tests/selector.rs` | §1318 |
| **K** | a text layer is a layer whose content is text, and it carries text animators | `docs/plans/effectcraft.md` | `craftkit-plugin-fx` | `src/text.rs` | `fx.layer.text`, `fx.layer.text.animator` | `crates/craftkit-plugin-fx/tests/text.rs` | §1318 |
| **L** | one layer may be the matte of another | `docs/plans/effectcraft.md` | `craftkit-plugin-fx` | `src/matte.rs` | `fx.layer.matte` | `crates/craftkit-plugin-fx/tests/matte.rs` | §1319 |
| **M** | an expression is written in a script language, not in the battery's arithmetic | `docs/plans/effectcraft.md` | `craftkit-plugin-fx` | `src/script.rs` | `fx.prop.expression.script` | `crates/craftkit-plugin-fx/tests/script.rs` | §1320 |
| **N** | the reader and the writer of the format are a shared library, and the battery's commands use it | `docs/plans/effectcraft.md` | `craftkit-lottie` | `src/lib.rs` | `lottie.read`, `lottie.write` | `crates/craftkit-lottie/tests/lottie.rs` | §1321 |
| **N** | a composition may be read from and written to the format | `docs/plans/effectcraft.md` | `craftkit-plugin-fx` | `src/lottie.rs` | `fx.lottie.import`, `fx.lottie.export` | `crates/craftkit-plugin-fx/tests/lottie.rs` | §1321 |
| **O** | a region may be brushed, segmented, and used as an object matte | `docs/plans/effectcraft.md` | `craftkit-plugin-fx` | `src/roto.rs` | `fx.roto.brush`, `fx.roto.segment`, `fx.matte.object` | `crates/craftkit-plugin-fx/tests/roto.rs` | §1322 |
| **P** | a composition may be tracked and stabilised | `docs/plans/effectcraft.md` | `craftkit-plugin-fx` | `src/track.rs` | `fx.track.analysis`, `fx.track.stabilise` | `crates/craftkit-plugin-fx/tests/track.rs` | §1323 |
| **Q** | a composition may hold footage, image sequences and audio | `docs/plans/effectcraft.md` | `craftkit-plugin-film` | `src/media.rs` | `film.media.footage`, `film.media.sequence`, `film.media.audio` | `crates/craftkit-plugin-film/tests/media.rs` | §1324 |
| **R** | a render may be queued, and its settings, output modules and encoders are named by the tree's own rasteriser | `docs/plans/effectcraft.md` | `craftkit-plugin-film` | `src/render.rs` | `film.render.queue`, `film.render.settings`, `film.render.output` | `crates/craftkit-plugin-film/tests/render.rs` | §1325 |
| **S** | a plug-in's own state is kept in the composition | `docs/plans/effectcraft.md` | `craftkit-plugin-fx` | `src/plugin_state.rs` | `fx.plugin.state` | `crates/craftkit-plugin-fx/tests/plugin_state.rs` | §1326 |

The second wave is the whole upstream: every row the survey held back — the
**later** rows and the **skipped** rows alike — is an arc above, and nothing in
the upstream's compositing surface is left behind. The survey's **later**
verdicts were judgements about **size**, not about **scope**: they said the work
was bigger than a single arc, and a bigger arc is still an arc, so each of them
is split or taken whole here.

A row that belongs to another battery is not dropped for belonging to it: it is
an arc **in** that battery. **J**'s paths land in the vector battery and the
shape layer itself in the fx battery; **K**'s text lands in the text battery and
the text layer and its animators in the fx battery; **N**'s format becomes a
shared library crate and the battery's commands that use it stay in the fx
battery; **Q** and **R** land in the film battery, media and render being
theirs. The encoders of **R** are **the tree's own rasteriser's dependency**,
named by the arc — the film battery asks the rasteriser to encode, and does
not bring an encoder of its own.

**§5 named no row outside this list**, so the thirteenth arc is the one this
list gave: the **plug-in state kept in the composition**.

## 3. The order, and what each arc ships

| arc | ships | cost |
|---|---|---|
| **A** | the layer's in point and out point, and the commands that set and read them | small — two fields on the layer, and the composition's own time is already there to bound them |
| **B** | stretch, reverse and freeze on those points | medium — three operations over the layer's time, and each one changes what the layer evaluates to |
| **C** | the mask's feather and its expansion | small — the mask node already keeps both fields, and only the commands were missing |
| **D** | the mask's opacity | small — the third field the mask node already keeps, and the same shape as C |
| **E** | a keyframe's spatial tangents, written down | medium — the auto tangents have to become a default that a command may override |
| **F** | the report of an expression's errors | medium — the errors have to be collected and read, not only refused |
| **G** | the catalogue of effects, each with its parameters, kinds and defaults | medium — the six the battery has are widened into a table the battery reads from |
| **H** | a layer's style group and the composition's global light | medium — a style group is a new thing on a layer, and every style needs the light |
| **I** | three-dimensional layers, cameras and lights | large — a new layer kind, and a camera and a light are new things in a composition |
| **J** | the shape's paths in the vector battery, and the shape layer in the fx battery | medium — the paths are new to the vector battery, the shape layer is a new kind |
| **K** | the range selector in the text battery, and the text layer and its animators in the fx battery | medium — a selector is a text-battery thing, and the animators read it |
| **L** | one layer as the matte of another | medium — a third relation between layers, and a rendering decision with it |
| **M** | an expression written in a script language | large — a second language, and the battery's own arithmetic stays beside it |
| **N** | the format as a shared library crate, and the battery's commands that use it | large — a new crate, its layer row, and its architecture row |
| **O** | the brush, the segmentation, and the object matte | medium — a painted region is a new thing to carry |
| **P** | tracking and stabilisation | large — an analysis that runs before any command runs |
| **Q** | footage, image sequences and audio, in the film battery | large — a media kind, and the film battery's own |
| **R** | the render queue, the settings, the output and the encoders, in the film battery | large — the rasteriser's dependency is named, and the queue outlives a command |
| **S** | the plug-in's own state in the composition | medium — the state is the plug-in's shape, and the document carries it |

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
| **G** | every entry of the catalogue carries its parameters, its kinds and its defaults, and a parameter the reader does not know is refused |
| **H** | a style group round-trips on its layer; the global light is the composition's and one light answers for every layer that has no light of its own |
| **I** | a three-dimensional layer keeps its own transform and its own depth; a camera and a light are named and read back, and an unknown one is refused |
| **J** | the paths round-trip through the vector battery; a trim, a repeat, a round corner, an offset and a gradient stroke each change what the path is; the shape layer holds the path it was given |
| **K** | a range selector names a range of a text run and refuses a range that is not there; the text layer holds its text, and its animators read the selector |
| **L** | a matte relation round-trips; the layer that is the matte and the layer that uses it are named both ways; a matte that would make a cycle is refused |
| **M** | a script expression is evaluated and yields the value the script language gives; an unknown name in the script is an error and not a panic |
| **N** | the shared library reads the format and writes it, and the two agree; the battery's import command puts what the reader read into a composition, and the export command writes what the composition holds |
| **O** | a brushed region is stored and read back; a segmentation is a region the analysis named; an object matte uses the region as the matte of a layer |
| **P** | a tracking run yields a track the composition keeps; a stabilisation applies the track; a track that cannot be found is refused |
| **Q** | footage, an image sequence and an audio clip are held by the composition; a file that is not there is refused, and a sequence with a gap is refused |
| **R** | a render is queued and the queue reads back in order; the settings and the output modules round-trip; the encoders named are the rasteriser's own dependency and no other |
| **S** | a plug-in's state round-trips through the composition; state belonging to a plug-in that is not linked in is refused |

## 5. What it does not do

**Nothing in the upstream's compositing surface is refused.** Every row the
survey held back is an arc above, and the rows that belonged to another battery
are arcs **in** that battery: the shape's **paths** in the vector battery, the
**text** in the text battery, the **media** and the **render** in the film
battery. A second battery's work is not a second copy of it — the vector
battery's paths are read once, and the fx battery's shape layer holds them.

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

**The second wave leaves one thing open, and it is the same one.** The survey
never read the vector, text and film batteries, so where the shape's paths,
the text, the media and the render land is planned from the batteries' names
and not from their code; whether one of them already offers what an arc above
would add is **unsure**. The second wave also assumes the tree's rasteriser
already names its own encoder dependency, which no survey read.
