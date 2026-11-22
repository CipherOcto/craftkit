# The survey of the batteries: what of each family tree is still not here

The grid port is done: §1097 surveyed the spreadsheet's reading surface
against the reference, §1098–§1100 checked that claim and fixed what the
check found. The owner's question was the obvious next one: **what is left
on all the other batteries?**

This is the answer, measured rather than remembered.

---

## The method

The tree's own rule for this kind of work is in the playbook (§8):

> **A sub-agent gets a bounded, mechanical brief, or it does not get
> spawned.** — *One survey per battery, with a written contract.*

So: **one contract per battery**, read-only, and the same five questions
each time — how many commands does the reference declare, what does its
document model hold, which formats does it read and write, what does the
battery have in the same places, and what does the battery have *no*
counterpart for. Every row cites a `file:line` on one side or the other,
and every row carries a verdict from a fixed vocabulary:

| the verdict | what it means |
|---|---|
| **read** | the battery has it |
| **refused** | the battery refuses it and says why — the reason is quoted |
| **coarse** | the battery has it in a deliberately coarser form |
| **out of scope** | it belongs to another battery, or to the front ends |
| **to judge** | nobody can tell from the two trees whether it is a decision or a gap |

The last one is the honest one, and the surveys used it a lot: **reading
what a rule means stays with the reader** (§8), so a survey points at a
difference and does not decide what it means. The verdicts below that say
**to judge** are the owner's list, not the survey's.

### What the counts do and do not say

The references are **applications** — a PDF editor, a video editor, a
word processor, a compositing app, a CAD program, a vector editor — and
the batteries are **models**: a document, a registry of commands over it,
and a shape that can be drawn. A reference's registry carries its **UI**
(`view.zoom`, `help.about`, `edit.undo`, `app.quit`); a battery's carries
the **document** (`pdf.page.add`, `film.clip.trim`). The counts are
therefore *not* one-to-one comparable, and the difference between them
says little on its own. **Table B — the features with no counterpart — is
the substance**; the counts are the frame.

### The counts are the tree's own

Where the tree has an index, the battery's side of the count is the
tree's own number, not the survey's estimate: `cargo xtask census` says
**pdf 119**, **prose 110**, **film 113**, **fx 47**, **cad 55**,
**vector 67**, **layout 73**, **sound 144**, and **art 11** — and the art
number was **wrong**: the art battery declares twelve of its commands
through a macro, and the index could not see a macro. §1102 fixed the
reader, and the census says **art 23** now, and **1036** commands in all
— the number the README has been saying all along.

---

## Table A — the counts, side by side

| the battery | its family tree | commands | model / node kinds | formats read | formats written |
|---|---|---|---|---|---|
| **pdf** | pdfcraft | 178 → **119** | 28 types → **3 kinds** | 8 → **5** | 10 → **5** |
| **prose** | wordcraft | 648 → **110** | 180 types → **10 kinds** | 19 → **0** | 14 → **0** |
| **film** | filmcraft | 579 → **113** | 110 types → **6 kinds** | 63 → **11** | 35 → **12** |
| **fx** | effectcraft | 439 → **47** | 131 types → **4 kinds** (310 effects → **6**) | 17 → **0** | 22 → **0** |
| **cad** | cadcraft | 426 → **55** | 30 entity kinds → **8** | 4 → **2** | 6 → **2** |
| **vector** | vectorcraft | 806 → **67** | 13 kinds → **1** | 22 → **1** | 20 → **2** |
| **sound** | soundcraft | 483 → **144** | 38 types → **16 kinds** + 18 payloads | 9 → **1** (WAV) | 0 → **0** |
| **art** | artcraft | 67 Tauri + 39 TS → **23** | 7 element kinds + 5 track kinds | 0 → **0** | 0 → **0** |
| **layout** | designcraft | 542 → **73** | 217 types → **13 kinds** | 12 → **0** | 9 → **0** |

The **formats** row is the one that stands out: **five batteries read and
write no file format of their own** — the prose, fx, art and layout, and
the vector's twenty — and the sound battery reads one, WAV. That is not an
oversight; the kit has its own format crates, and the tree says so where
it matters (`craftkit-xlsx` is the spreadsheet's, `craftkit-plugin-pdf`
the PDF's, `craftkit-plugin-vector` the vector's).

---

## The batteries, one by one

### The PDF battery — pdfcraft

The battery is a faithful port of the reference's **object model** — the
COS layer, the dictionaries, the streams — and it deliberately has no
rasteriser, no UI and no export to other formats.

| the reference's area | the battery | the verdict |
|---|---|---|
| page rasterisation (`crates/render/src/raster.rs:188`) | none — "A PDF object is **not drawn**" (`src/lib.rs:310`) | out of scope |
| export to Word, HTML, RTF (`crates/engine/src/commands.rs:285-287`) | none | to judge |
| OCR (`crates/ocr/src/lib.rs:91`) | none | to judge |
| XFA dynamic forms (`crates/xfa/src/lib.rs:77`) | none | to judge |
| measurement (`crates/measure/src/lib.rs:228`) | none | to judge |
| digital signing (`crates/sign/src/lib.rs:33`) | a signature *field* and a read (`src/signaturefield.rs:59`) | coarse |
| page assembly — import, extract, split, combine (`crates/organize/src/import.rs:189`) | add and remove a page (`src/lib.rs:864`, `src/pages.rs:48`) | coarse |
| bookmark editing (`crates/organize/src/outline.rs:226-279`) | add and read (`src/outline.rs:58,290`) | coarse |
| field actions and scripts (`crates/forms/src/scripting.rs:203`) | "a script is **read and not run**" (`src/javascript.rs:12`) | coarse |
| JavaScript execution (`crates/js/src/lib.rs:1076`) | the same note | coarse |
| accessibility report (`crates/a11y/src/report.rs:11`) | the structure tree is read and alt text set (`src/structure.rs:159`) | coarse |
| creating documents (`crates/create/src/lib.rs:59`) | a file is loaded (`src/lib.rs:1436`) | to judge |

### The prose battery — wordcraft

The reference is a word processor with **648** commands and a document
model of **180** types; the battery has **110** commands and **10** node
kinds, and — this is the fact — **no file format at all**: its manifest
depends on the SDK and the derive macro and nothing else
(`crates/craftkit-plugin-prose/Cargo.toml:10-15`).

| the reference's area | the battery | the verdict |
|---|---|---|
| file formats — 19 read, 14 written (`crates/engine/src/io.rs:9,31`) | none | to judge |
| proofing — spelling, grammar, thesaurus (`cmd/review.rs:81,133,139`) | none | to judge |
| content controls (`crates/doc/src/control.rs:224`) | none; the tree's README names it as not carried (`README.md:4675`) | to judge |
| document compare (`crates/doc/src/compare.rs:1`) | none | to judge |
| track changes — 35 `review.*` ids (`cmd/review.rs:47`) | accept and reject (`src/resolving.rs:138`) | coarse |
| threaded comments (`cmd/review.rs:28-46`) | a comment is a block-anchored text record (`src/comments.rs:1-16`) | coarse |
| headers and footers as separate stories (`crates/doc/src/parts.rs:63`) | "the running head is not a paragraph, and it has no style of its own here" (`src/running.rs:22`) | coarse |
| sections and columns (`crates/doc/src/section.rs:723`) | one page per document (`src/page.rs:147`) | coarse |
| nested table cells (`crates/doc/src/table.rs`) | "the port of the nesting is **not built**" (`src/lib.rs:78-88`) | coarse |
| equations — 21 ids (`cmd/equation.rs:31`) | two commands (`src/equations.rs:150`) | coarse |
| fields | "the words a field covers are its result, and they are worked out every time" (`src/fields.rs:38-46`) | coarse |
| citations and bibliography (`cmd/citations.rs:37`) | a fixed style list and a local source table (`src/citestyles.rs:17`) | coarse |
| drawings — shapes, WordArt, SmartArt, ink (`crates/doc/src/smart_art.rs:212`) | none | out of scope |
| layout and pagination (`crates/layout`) | the prose battery stores text and never breaks a line | out of scope |

### The film battery — filmcraft

The reference is a video editor with **579** commands; the battery has
**113** and six node kinds, and it is **ahead** in one place — it reads
the ALE format the reference only writes.

| the reference's area | the battery | the verdict |
|---|---|---|
| graphics and titles — 77 ids (`crates/engine/src/graphics.rs:964`) | test-card generators only (`src/generators.rs:63`) | to judge |
| the mixer and its automation (`crates/project/src/mixer.rs:172`) | a track carries a volume and a pan (`src/tracks.rs:137,174`) | coarse |
| multicam — 15 ids (`crates/engine/src/multicam.rs:178`) | a clip synchronised to another clip (`src/anchors.rs:115`) | coarse |
| effects and keyframes (`crates/project/src/effect.rs:105`) | "a clip's **effects** are its own, and they live in its payload" (`src/lib.rs:177-179`) | coarse |
| media — 49 extensions read, 21 written (`crates/media/src/lib.rs:229`) | a media path is an opaque string (`src/lib.rs:544`) | coarse |
| trim — 23 ids | one trim, one edge at a time (`src/trim.rs:89`) | coarse |
| markers — 32 ids (`cmd/sequence_tools.rs:225`) | add and set (`src/markers.rs:94,183`) | coarse |
| captions, transcripts, speech (`crates/engine/src/transcript.rs:439`) | caption files are read and written (`src/captions.rs:750`) | coarse |
| colour management (`crates/engine/src/color.rs:32`) | none | to judge |
| export queue and render (`crates/engine/src/export_tools.rs:1161`) | none | out of scope |
| project file and autosave (`crates/format/src/lib.rs:1`) | none — the battery has no document file of its own | to judge |

### The fx battery — effectcraft

The reference is a whole compositing application: **439** commands, **310**
built-in effects, a full AE expression language on a JavaScript engine, and
a render queue with thirteen media output formats. The battery is a
**model**: four node kinds, six effects, and an arithmetic language.

| the reference's area | the battery | the verdict |
|---|---|---|
| the effect catalogue — 310 ids (`crates/effects/src/lib.rs:125`) | six (`src/effects.rs:133`) | coarse |
| the expression language (`crates/expr/src/lib.rs:1-28`) | "a small arithmetic language" (`src/lib.rs:55-60`) | coarse |
| markers — nine fields (`crates/project/src/settings.rs:278`) | "only the time and the comment are here" (`src/markers.rs:7-13`) | coarse |
| spatial keyframes (`crates/keyframe/src/lib.rs:73-79`) | "the spatial tangents are not written down" (`src/keys.rs:84-88`) | coarse |
| layer model — 21 fields (`crates/project/src/layer.rs:281`) | three payload keys (`src/lib.rs:285`) | coarse |
| switches — 14 (`crates/project/src/layer.rs:146`) | five (`src/switches.rs`) | coarse |
| keyframe labels, smoothing, presets (`cmd/anim_tools.rs:400`) | none | to judge |
| property-tree groups (`cmd/prop.rs`) | one path (`src/lib.rs:301`) | to judge |
| track mattes, layer timing, frame blending (`cmd/layer_menu.rs:925`) | none | to judge |
| picture analysis — tracking, roto, paint, liquify, puppet (`cmd/track.rs:1`) | none | out of scope |
| interchange formats and Lottie (`crates/interchange/src/lib.rs:57`) | none | to judge |
| render queue (`cmd/render_queue.rs:1`) | none | out of scope |

### The cad battery — cadcraft

The reference has **426** commands and a **closed** enum of thirty entity
kinds; the battery has **55** commands over an **open** kind system with
**eight** entity kinds — line, circle, arc, text, insert, ellipse,
polyline and dimension (`src/lib.rs:266-272`, `src/dimensions.rs:39`). The
tree says why it is open: "a new entity is a **new kind**"
(`src/lib.rs:52-56`).

| the reference's area | the battery | the verdict |
|---|---|---|
| the entity union — thirty kinds (`crates/doc/src/entity.rs:63-94`) | eight | coarse |
| the constraint solver (`crates/constraints/src/lib.rs:459`) | "**The geometry is not moved.**" (`src/constraints.rs:14-16`) | coarse |
| dimensions — eight kinds (`crates/doc/src/entity/dim.rs:8`) | one: the length of an entity (`src/dimensions.rs:96`) | coarse |
| DXF import — 30 kinds (`crates/io/src/dxf_read.rs`) | five: LINE, CIRCLE, ARC, INSERT, LWPOLYLINE (`src/dxfimport.rs:300-357`) | coarse |
| blocks as blocks (`crates/io/src/dxf_write.rs`) | "The **blocks** of a drawing are not written as blocks" (`src/dxfexport.rs:23-25`) | coarse |
| the audit (`cmd/audit.rs:15`) | "the audit **reports and does not repair**" (`src/audits.rs:17-18`) | coarse |
| the current layer (`cmd/layer.rs:24`) | "an entity names the layer it is on" (`src/layersettings.rs:21-22`) | coarse |
| the twenty-two missing entity kinds — spline, hatch, mtext, leader, region, solid, image, tolerance … | none | to judge |
| style tables — text, dimension, table, plot, mleader (`crates/doc/src/lib.rs:92-100`) | layers and linetypes only | to judge |
| layouts and paper space (`cmd/layout.rs:27`) | one space | to judge |
| modify — trim, extend, fillet, chamfer, offset (`cmd/modify.rs:81-98`) | none | to judge |
| DWG, PNG, PDF, DWT (`crates/doc/src/format.rs:19-21`) | DXF and SVG | to judge |
| undo marks and groups (`cmd/edit.rs:17`) | the kit's own history (`crates/craftkit-sdk/src/history.rs:417`) | out of scope |

### The vector battery — vectorcraft

The reference has **806** commands and **thirteen** node kinds; the
battery has **67** commands and **one** kind, `vector.path`
(`src/lib.rs:282`).

| the reference's area | the battery | the verdict |
|---|---|---|
| the appearance stack — fills and strokes, each with its own opacity, blend and effects (`crates/doc/src/appearance.rs:1-2`) | one fill and one stroke (`src/lib.rs:190-192`) | coarse |
| the live families — blend, envelope, mesh, repeat, compound, symbol instance (`crates/doc/src/node/kind.rs:74-122`) | none of them | to judge |
| the file surface — 24 formats, 22 read, 20 written (`cmd/fileio/formats.rs:344`) | one read, two written | to judge |
| the anchor-based path model (`crates/geom/src/path.rs:33`) | segments, with anchors converted on demand (`src/convert.rs:31`) | coarse |
| SVG reading (`crates/svg/src/import.rs`) | "an element this tree cannot read — a shape, a text, a gradient — is **skipped**" (`src/svgimport.rs:17-18`) | coarse |
| SVG writing (`crates/svg/src/export.rs`) | "the **transform** of a path is **baked** into it" (`src/svgexport.rs:16-17`) | coarse |
| EPS writing (`crates/eps/src/print.rs`) | "the **transparency** of the tree is not here: EPS has none" (`src/epsexport.rs:35-36`) | coarse |
| colour management — CMYK, spot inks, overprint (`crates/doc/src/cmyk.rs:23`) | colour maths and a gamut (`src/colour.rs:395`) | coarse |
| patterns — a tiling fill (`crates/doc/src/pattern.rs:164`) | "Solid or Gradient only" (`crates/craftkit-sdk/src/document.rs:138-143`) | to judge |
| artboards, slices, print tiling (`cmd/layer.rs:126`, `cmd/slices.rs:143`) | none | to judge |
| effects, opacity and clipping masks (`cmd/opacitymask.rs:22`) | none | to judge |

### The sound battery — soundcraft

The reference is a digital audio workstation with **483** commands; the
battery has **144** and sixteen node kinds, and it is the battery whose
own prose says the most about its decisions — the sound model's refusals
are written out one by one (`src/lib.rs:87-160`).

| the reference's area | the battery | the verdict |
|---|---|---|
| session interchange — AAF, OMF, `.ptx` (`docs/gaps.md:17`) | none | to judge |
| MIDI device input and output (`docs/gaps.md:18`) | none | to judge |
| compressed audio — MP3, AAC/M4A, Ogg/Opus (`docs/gaps.md:33`) | the battery refuses rather than reads (`crates/craftkit-wav/src/lib.rs:146`) | refused |
| the pan law — a per-channel constant-power panner (`cmd/mix.rs:13`) | one pan that balances (`src/lib.rs:118-127`) | coarse |
| the audio suite as a rendering (`cmd/audiosuite.rs:12`) | "a process is a name on the clip, and the sound is worked out when it is played" (`src/lib.rs:129-137`) | coarse |
| speed and warp as two things (`cmd/clip_more.rs:67`) | one stretch (`src/lib.rs:150-160`) | coarse |
| the track kind as a closed set (`crates/model/src/track.rs:8`) | "a track's kind is a **string in the track's payload**" (`src/lib.rs:87-90`) | coarse |
| the clip content as a closed set (`crates/model/src/clip.rs:85`) | "the clip says what its content is with a **string**" (`src/lib.rs:175-177`) | coarse |
| insert parameters, one by one (`cmd/mix.rs:100-101`) | one JSON blob (`src/mixer.rs:294-300`) | coarse |
| solo modes — SIP, AFL, PFL, latch (`cmd/options.rs:45`) | solo and safe solo (`src/lib.rs:1562,1921`) | coarse |
| tempo curve shapes (`cmd/event_more.rs:45-72`) | a tempo node holds a tick and a bpm (`src/lib.rs:5885-5890`) | coarse |
| marker kinds and memory locations (`crates/model/src/markers.rs:7,19`) | a marker holds a start and an end (`src/lib.rs:5936-5941`) | coarse |
| time formats — timecode, feet+frames (`crates/time/src/format.rs:17`) | three (`crates/craftkit-sound-time/src/lib.rs:450-457`) | coarse |
| the track's output routing (`cmd/track.rs:204`) | the input and the monitor are there (`src/mixer.rs:519`, `src/recording.rs:76`) | to judge |
| automation lane copy and paste (`cmd/edit_more.rs:73-74`) | lanes take points (`src/mixer.rs:1218`) | to judge |
| click and count-off (`cmd/setup_more.rs:75`) | the click is mixed, not commanded | to judge |
| bounce, render, stems (`cmd/file.rs:90-91`) | `sound.render`, `sound.render.stems` (`src/lib.rs:4849,4891`) | read |
| MIDI import and export (`cmd/file.rs:69,107`) | `sound.track.midi.import`, `sound.clip.midi.export` (`src/midi.rs:511,615`) | read |
| the sync point and the beat (`cmd/clip.rs:104,115`) | `sound.clip.sync.set` and `.remove` (`src/beat.rs:70,153`) | read |
| the grid and the snap (`cmd/edit.rs:159`) | `sound.grid.set`, `sound.clip.quantize` (`src/grid.rs:60,168`) | read |
| auto-rename (`cmd/auto_rename.rs:13`) | `sound.clip.autoname` (`src/autoname.rs:221`) | read |
| undo, selection, tools, the view, plugin hosting | the kit's own core and the front ends' | out of scope |

### The art battery — artcraft

The reference is a video editor written as a desktop application: **67**
Tauri commands, **39** more in the editor's own TypeScript layer, and a
Rust command surface of **63**. The battery has **23** commands over
seven element kinds and five track kinds, and it ports the **document**,
not the shell.

| the reference's area | the battery | the verdict |
|---|---|---|
| element mutation — update, move, delete (`video-editor/src/lib/commands/timeline/element/update-elements.ts:10`) | an element is added and timed (`src/lib.rs:456-525`) | to judge |
| the element's effect, mask, keyframe and parameter stack (`video-editor/src/lib/masks/types.ts:11`, `params/registry.ts:20`) | "the params, the effects, the masks, the keyframes | the element's **payload**" (`src/lib.rs:29`) | coarse |
| the animation channels — fourteen animatable paths (`video-editor/src/lib/animation/types.ts:4-19`) | the same payload | coarse |
| the 3D stage (`pagescene/src/lib/actions/index.ts:1-16`) | one opaque file beside the document (`src/lib.rs:222-223`) | coarse |
| the media library (`…/commands/media/add-media-asset.ts:6`) | "The media it plays: a token, not a file" (`src/lib.rs:464`) | coarse |
| the generation commands — image, video, mesh, audio, splat (`…/generate/generate_image/generate_image_command.rs:28`) | one `art.generate` (`src/lib.rs:713-755`) | coarse |
| the provider and credential surface (`…/provider_set_api_key_command.rs:23`) | a model name and a prompt (`src/lib.rs:716`) | coarse |
| the task queue (`…/task_queue/get_task_queue_command.rs:84`) | the kit's own tasks (`crates/craftkit-sdk/src/taskcmds.rs:31`) | coarse |
| the app's preferences and platform (`…/app_preferences/get_app_preferences_command.rs:48`) | none | out of scope |
| export to mp4 and webm (`video-editor/src/lib/export/index.ts:11`) | none | to judge |
| track mute, visibility and removal (`…/toggle-track-mute.ts:7`, `…/remove-track.ts:5`) | none — `art.track.*` has no payload | to judge |
| scene rename, delete and bookmarks (`…/scene/rename-scene.ts:6`, `…/toggle-bookmark.ts:11`) | `art.scene.add` only (`src/lib.rs:286`) | to judge |
| the canvas presets and the background (`video-editor/src/lib/canvas/sizes.ts:3`, `project/types.ts:5`) | none | to judge |
| the clipboard (`…/timeline/clipboard/paste.ts:24`) | none | to judge |
| the cost estimate (`…/cost_estimate/estimate_image_cost_command.rs:21`) | none | to judge |
| the guides and the safe area (`…/guides/registry.ts:12`) | none | to judge |
| the thumbnail, the mastering, the waveforms (`…/media/audio-mastering.ts:66`) | none | to judge |
| the undo and redo of a command (`…/commands/base-command.ts:28`) | the kit's session history (`crates/craftkit-sdk/src/history.rs:457`) | read |
| the selection of elements, mask points and keyframes (`…/selection/hit-testing.ts:31`) | the kit's node and span selection (`crates/craftkit-sdk/src/select.rs:78-131`) | coarse |
| the rate bounds — 0.01 to 5 (`…/retime/rate.ts:2-3`) | the same bounds, clamped (`src/lib.rs:74-77`) | read |
| a trim past the end of the media | "the media is {source} ticks long and the trim ends at {to}" (`src/lib.rs:891-895`) | refused |
| an element of zero length | "an element is not zero ticks long" (`src/lib.rs:417-422`) | refused |
| a ripple of nothing | "a ripple takes some space out or it takes none" (`src/lib.rs:965-970`) | refused |

### The layout battery — designcraft

The reference is a page-layout application: **542** commands and **217**
document types. The battery has **73** commands and **13** node kinds, and
it declares no format of its own — `layout.asset.add` records a path
string and nothing more (`src/lib.rs:740`).

| the reference's area | the battery | the verdict |
|---|---|---|
| character and paragraph formatting — sixty `type.*` ids (`crates/engine/src/cmd/text.rs:204-244`) | a story is one text field (`src/linked.rs:79`) | coarse |
| the table model — forty ids (`crates/engine/src/cmd/table/registry.rs:81`) | no table kind | to judge |
| the export family — PDF, EPUB, HTML, PPTX, text (`crates/engine/src/cmd/export/registry.rs:7-49`) | none | to judge |
| object transforms — move, resize, shear, flip (`crates/engine/src/cmd/object.rs:41-124`) | the rect is set at creation (`src/lib.rs:805`) | coarse |
| the effects family — eleven kinds (`crates/engine/src/cmd/object.rs:380-591`) | one shadow (`src/shadows.rs:116`) | coarse |
| gradients and swatches (`crates/engine/src/cmd/style.rs:225`) | "the tree's fill and stroke are swatches … Here a colour is a colour" (`src/paint.rs:23-27`) | coarse |
| stroke kinds, caps, joins (`crates/engine/src/cmd/object.rs:257`) | "a stroke is a colour and a weight" (`src/paint.rs:28-31`) | coarse |
| the corner options (`crates/engine/src/cmd/object.rs:606`) | one radius, refused beyond half the shorter side (`src/radii.rs:14-16`) | coarse |
| parent pages and overrides (`crates/engine/src/cmd/overrides.rs:14`) | one parent edge (`src/parents.rs:126`) | coarse |
| margins and columns (`crates/engine/src/cmd/layout/registry.rs:250`) | two commands (`src/margins.rs:94`, `src/columns.rs:89`) | read |
| CJK typography — ruby, tate-chu-yoko (`crates/engine/src/cmd/text.rs:295-322`) | none | to judge |
| text frames and threading (`crates/engine/src/cmd/thread.rs:16`) | none | to judge |
| IDML interchange (`crates/engine/src/cmd/interchange.rs:19-22`) | none | to judge |
| XML tagging (`crates/engine/src/cmd/xml.rs:184-303`) | none | to judge |
| sections and numbering (`crates/engine/src/cmd/layout/registry.rs:525-559`) | none | to judge |
| find and replace, data merge, variables, index and contents (`crates/engine/src/cmd/find.rs:66`, `cmd/datamerge.rs:79`, `cmd/variables.rs:26`, `cmd/index.rs:23`) | none | to judge |
| colour management and prepress (`crates/engine/src/cmd/colorsettings.rs:44`) | the tree puts it in `craft-libs` (`README.md:3022`) | out of scope |
| printing and preflight (`crates/engine/src/cmd/print.rs:12`) | the tree puts it in `craft-libs` (`README.md:3027`) | out of scope |

---

## What is left, in four lists

### One: deliberately coarse, in the tree's own words

This is the largest class, and it is **not debt**. The applications are
programs; the batteries are models. A tree that says so is a tree that has
decided, and the tree's rule is that a decision is written down where it
is made. The clearest statements, each in the battery's own prose:

- **The PDF object is not drawn** (`crates/craftkit-plugin-pdf/src/lib.rs:310`).
- **A script is read and not run** (`crates/craftkit-plugin-pdf/src/javascript.rs:12`).
- **The geometry is not moved** (`crates/craftkit-plugin-cad/src/constraints.rs:14-16`).
- **The audit reports and does not repair** (`crates/craftkit-plugin-cad/src/audits.rs:17-18`).
- **A clip's effects live in its payload** (`crates/craftkit-plugin-film/src/lib.rs:177-179`).
- **The tree's expressions are a small arithmetic language** (`crates/craftkit-plugin-fx/src/lib.rs:55-60`).
- **The tree's marker has only the time and the comment** (`crates/craftkit-plugin-fx/src/markers.rs:7-13`).
- **An element this tree cannot read is skipped, and the answer says what it skipped** (`crates/craftkit-plugin-vector/src/svgimport.rs:17-18`).
- **The transform of a path is baked into it** (`crates/craftkit-plugin-vector/src/svgexport.rs:16-17`).
- **The port of the nesting is not built** (`crates/craftkit-plugin-prose/src/lib.rs:78-88`).
- **A track's kind is a string in the track's payload** (`crates/craftkit-plugin-sound/src/lib.rs:87-90`).
- **A process is a name on the clip** (`crates/craftkit-plugin-sound/src/lib.rs:129-137`).
- **The media an element plays is a token, not a file** (`crates/craftkit-plugin-art/src/lib.rs:464`).
- **The 3D scene is one opaque file beside the document** (`crates/craftkit-plugin-art/src/lib.rs:222-223`).
- **A colour is a colour, and a stroke is a colour and a weight** (`crates/craftkit-plugin-layout/src/paint.rs:23-31`).

### Two: refused, with a reason

The tree refuses rather than guesses (§ the house rules), and it says
what it refuses:

- the **regular expressions** and the **cube functions** of the grid
  battery — `not supported yet: a regular expression`;
- a **format that is not a caption file's** — the film battery names the
  list it knows (`crates/craftkit-plugin-film/src/captions.rs:490-496`);
- a **compressed audio file** — the sound battery names the WAV it reads
  (`crates/craftkit-wav/src/lib.rs:146`);
- an **element of zero ticks**, a **trim past the end of the media** and a
  **ripple of nothing** — the art battery gives the reason in each case
  (`crates/craftkit-plugin-art/src/lib.rs:417-422,891-895,965-970`);
- a **radius beyond half the shorter side** — the layout battery refuses
  rather than clamps (`crates/craftkit-plugin-layout/src/radii.rs:14-16`).

### Three: out of scope

The **front ends'** own commands — zoom, pan, undo, shortcuts, help,
scripts — are the kit's camera, its history and its doors, not a battery's.
And the areas that belong to a **sibling battery**: the PDF's raster and
its exports are the image and pdf batteries'; the vector's text and images
are the text and image batteries'; the film's graphics and titles are the
art battery's; the prose's drawings are the shapes battery's; the
layout's typography is the text battery's; the art battery's app
preferences are the shell's.

### Four: to judge — the owner's list

These are the rows where **the tree is silent**: nothing in the battery
says the omission is a decision, and nothing says it is an oversight. The
survey may not decide them (§8), so they are here, and they are the
honest answer to the question the owner asked.

| the battery | the areas that are to judge |
|---|---|
| **pdf** | export to other formats, OCR, XFA forms, measurement, creating documents |
| **prose** | file formats, proofing, content controls, document compare, templates, autoformat |
| **film** | graphics and titles, colour management, the project file, transcoding, media browsers |
| **fx** | keyframe labels and presets, property groups, track mattes, layer timing, interchange formats |
| **cad** | the twenty-two missing entity kinds, the style tables, layouts and paper space, the modify commands, DWG and the other formats |
| **vector** | the live families, patterns and tiling fills, artboards and slices, effects and masks, symbols and libraries |
| **sound** | session interchange, MIDI device input and output, the track's output routing, automation lane copy and paste, click and count-off |
| **art** | element mutation and removal, export, track mute and visibility, scene rename, delete and bookmarks, canvas presets, backgrounds, the clipboard, cost estimation, guides, thumbnails, media mastering |
| **layout** | the table model, the export family, CJK typography, text frames and threading, IDML and XML tagging, sections and numbering, find and replace, data merge, variables, index and contents |

---

## The next arcs, and what the survey says about them

The survey's own finding, which is the useful part:

> **The remaining port work is in the batteries' *models*.** The
> applications' **analysis** (tracking, roto, OCR, rasterisation), their
> **rendering** (the render queue, the media encoders) and their **UI**
> (zoom, help, shortcuts) are not a battery's business, and the batteries
> that have a place for them say so.

So the arcs that are worth doing, largest first:

1. **The cad battery's missing entity kinds** — twenty-two kinds, each a
   node kind, a payload and a DXF mapping. It is the largest single model
   gap in the tree, and it is mechanical.
2. **The art battery's element commands** — move, update and delete an
   element, and the track's mute and visibility. The battery can add an
   element and cannot change one.
3. **The vector battery's live families** — blend, envelope, mesh, repeat,
   compound — plus the patterns and the artboards.
4. **The film's mixer** — strips, sends, automation — and its graphics and
   titles.
5. **The fx battery's model** — property groups, keyframe labels, layer
   timing, track mattes.
6. **The prose battery's model** — the proofing, the compare, the content
   controls, the nested table cells.
7. **The layout battery's table model** — a table in a page, which the
   reference has forty commands for.
8. **The sound battery's routing** — the output side, the session
   interchange, the count-off.
9. **The file formats** — and here the tree has already said the word
   (§899): **"The file formats are a project and not an arc."** A
   half-ported format is worse than no format: a document that opens and
   loses half of itself is a document that lies.

---

## What this survey does not settle

- **It is a survey of the batteries, not of the behaviour.** A command
  that is here may answer differently than the reference's command of the
  same name — the tree's `SUBTOTAL` and the reference's `SUBTOTAL` are
  not the same function (§898), and that is a kind of difference the
  survey cannot count.
- **The counts are static.** They come from the registries as they are
  written, not from a run: a command registered by a path that is not a
  literal would be missed. The art battery's twelve macro-declared
  commands were exactly such a case, and §1102 made the index read
  them.
- **Each survey read one battery against one family tree.** A battery may
  have more than one family tree's worth of behaviour in it, and a family
  tree may have grown since it was surveyed.
- **The verdicts are the survey's, and the readings are the owner's.** A
  row marked **to judge** is a row where the trees do not say, and this
  document does not say either.
