# The artcraft plan: what the upstream has and the art battery does not

This plan is a survey of one upstream tree against the art battery, written
before any of it is built. It names eighteen arcs of future work, in order, in
the sections §1241 through §1351, and says plainly what the upstream has that
the battery does not.

## 0. The rule

Every arc serves one rule: **a capability the upstream has is a command in
the battery that owns it, and a capability the upstream does not have is
not.**

## 1. What the survey found

The survey read the upstream at `/mnt/data/mmacedoeu/work/tools/artcraft` and
compared it with `craftkit-plugin-art` as it stands. Nine capabilities are
marked PORT: they are commands to port, and they are the **first wave**. Six
are marked LATER: they are whole models rather than commands, and they are
the **second wave**. One is marked SKIP: the upstream's own application
pages, which belong to other batteries — and they are arcs there, not gaps
here.

| capability | upstream path:line | craftkit's state | verdict |
|---|---|---|---|
| Duplicating a timeline element (clones animations, re-places it) | `artcraft/frontend/libs/components/video-editor/src/lib/commands/timeline/element/duplicate-elements.ts:1-13` | absent | PORT |
| Splitting an element at a point in time | `artcraft/frontend/libs/components/video-editor/src/lib/commands/timeline/element/split-elements.ts:1-12` | absent | PORT |
| Track mute, track visibility, track removal | `artcraft/frontend/libs/components/video-editor/src/lib/commands/timeline/track/toggle-track-mute.ts:7`, `toggle-track-visibility.ts:7`, `remove-track.ts:5` | absent — the battery's track commands carry no payload | PORT |
| Removing a scene, and the scene's bookmarks | `artcraft/frontend/libs/components/video-editor/src/lib/commands/scene/delete-scene.ts:6`, `toggle-bookmark.ts:11` | partial — the battery can only add a scene | PORT |
| The element's effect stack (add, remove, reorder, toggle, set a parameter) | `artcraft/frontend/libs/components/video-editor/src/lib/commands/timeline/element/effects/add-effect.ts:8`, `remove-effect.ts`, `reorder-effect.ts`, `toggle-effect.ts` | absent — effects are only a name on the element | PORT |
| Timeline clipboard: copy and paste elements and keyframes | `artcraft/frontend/libs/components/video-editor/src/lib/commands/timeline/clipboard/paste.ts:1-13` | absent | PORT |
| Project settings and canvas presets (canvas size, fps, background) | `artcraft/frontend/libs/components/video-editor/src/lib/commands/project/update-project-settings.ts:5`, `artcraft/frontend/libs/components/video-editor/src/lib/commands/canvas/sizes.ts:3,10` | partial — the battery sets fps and ticks per second, no canvas size | PORT |
| Background colour and blur behind the composition | `artcraft/frontend/libs/components/video-editor/src/lib/commands/background/color.ts:1`, `blur.ts` | absent | PORT |
| Guides (thirds, grid, safe area) drawn over the composition | `artcraft/frontend/libs/components/video-editor/src/lib/commands/guides/registry.ts:12` | absent | PORT |
| The media library: add an asset, remove an asset | `artcraft/frontend/libs/components/video-editor/src/lib/commands/media/add-media-asset.ts:6`, `remove-media-asset.ts:12` | absent — the battery says only that media is "a token, not a file" | LATER |
| Keyframes on an element's animatable parameters | `artcraft/frontend/libs/components/video-editor/src/lib/commands/timeline/element/keyframes/upsert-keyframe.ts:1`, `retime-keyframe.ts`, `remove-keyframe.ts` | absent | LATER |
| Masks on an element (freeform points, invert, feather, dimensions) | `artcraft/frontend/libs/components/video-editor/src/lib/commands/timeline/element/masks/toggle-mask-inverted.ts:8`, `insert-custom-mask-point.ts` | absent | LATER |
| Export of the composition to a video file (mp4, webm, quality presets) | `artcraft/frontend/libs/components/video-editor/src/lib/commands/export/index.ts:4,11` | absent | LATER |
| Per-model generation parameters and a cost estimate before generating | `artcraft/crates/api_clients/artcraft/artcraft_client/src/endpoints/generate/cost_estimate/image/estimate_image_cost.rs` | partial — `art.generate` takes a model name and a prompt, no parameters, no estimate | LATER |
| Drawing canvas, image-to-3D mesh, background removal, watermark removal, video frame extraction, storyboard and moodboard pages | `artcraft/frontend/apps/artcraft/app/src/pages/` | absent from the art battery; craftkit has image, fx, film and vector batteries | SKIP |

The nine PORT rows are the arcs of §2. Three of them are split, because each
holds more than one command: the track row becomes three arcs, the scene row
becomes two, and the project settings row becomes two.

The later work is the media library, keyframes on an element's parameters,
masks, video export, and per-model generation parameters with a cost estimate.
Each of those is a model or a dependency before it is a command, and each of
them is an arc of the second wave in §2, in the battery that owns it.

## 2. The arcs

Each arc is three edits: a row in the port manifest, a row in the README's claim
table, and a section in the README. The sections reserved for this plan are
**§1241 through §1351**, one per arc, in the order below.

The first wave is the survey's PORT rows.

| arc | rule | from | to | module | command | test | section |
|---|---|---|---|---|---|---|---|
| **1** | an element is duplicated on its own, animations and all | `docs/plans/artcraft.md` | `craftkit-plugin-art` | `src/duplicate.rs` | `art.element.duplicate` | `crates/craftkit-plugin-art/tests/duplicate.rs` | §1241 |
| **2** | an element is split at a point in time | `docs/plans/artcraft.md` | `craftkit-plugin-art` | `src/split.rs` | `art.element.split` | `crates/craftkit-plugin-art/tests/split.rs` | §1242 |
| **3** | a track can be muted, and muting is a flag on the track | `docs/plans/artcraft.md` | `craftkit-plugin-art` | `src/track.rs` | `art.track.mute` | `crates/craftkit-plugin-art/tests/track_mute.rs` | §1243 |
| **4** | a track can be hidden, and hiding is a flag on the track | `docs/plans/artcraft.md` | `craftkit-plugin-art` | `src/track.rs` | `art.track.visibility` | `crates/craftkit-plugin-art/tests/track_visibility.rs` | §1244 |
| **5** | a track can be removed, and removing it takes its elements with it | `docs/plans/artcraft.md` | `craftkit-plugin-art` | `src/track.rs` | `art.track.remove` | `crates/craftkit-plugin-art/tests/track_remove.rs` | §1245 |
| **6** | a scene can be removed, not only added | `docs/plans/artcraft.md` | `craftkit-plugin-art` | `src/scene.rs` | `art.scene.remove` | `crates/craftkit-plugin-art/tests/scene_remove.rs` | §1246 |
| **7** | a scene carries its bookmarks, and a bookmark is toggled | `docs/plans/artcraft.md` | `craftkit-plugin-art` | `src/scene.rs` | `art.scene.bookmark` | `crates/craftkit-plugin-art/tests/scene_bookmark.rs` | §1247 |
| **8** | an element's effects are a stack, and the stack is edited | `docs/plans/artcraft.md` | `craftkit-plugin-art` | `src/effects.rs` | `art.effect.add`, `art.effect.remove`, `art.effect.reorder`, `art.effect.toggle`, `art.effect.set` | `crates/craftkit-plugin-art/tests/effects.rs` | §1248 |
| **9** | elements and keyframes are copied and pasted on the timeline | `docs/plans/artcraft.md` | `craftkit-plugin-art` | `src/clipboard.rs` | `art.clipboard.copy`, `art.clipboard.paste` | `crates/craftkit-plugin-art/tests/clipboard.rs` | §1249 |
| **10** | the composition has a canvas size, and the size is a setting | `docs/plans/artcraft.md` | `craftkit-plugin-art` | `src/canvas.rs` | `art.canvas.size` | `crates/craftkit-plugin-art/tests/canvas.rs` | §1250 |
| **11** | the project's settings are one record, and the record is edited | `docs/plans/artcraft.md` | `craftkit-plugin-art` | `src/project.rs` | `art.project.settings` | `crates/craftkit-plugin-art/tests/project.rs` | §1251 |
| **12** | what is drawn around the composition — the background and the guides — is a setting | `docs/plans/artcraft.md` | `craftkit-plugin-art` | `src/background.rs` | `art.background.color`, `art.background.blur`, `art.guides.set` | `crates/craftkit-plugin-art/tests/background.rs` | §1252 |

These arcs are twelve because the survey marked nine capabilities PORT and three
of them hold more than one command: the track row is mute, visibility and
removal, the scene row is the removal and the bookmarks, and the project
settings row is the canvas size and the rest of the settings. Arc 8 is the
only arc with five command ids, because add, remove, reorder, toggle and set
are five commands over one stack. Arc 12 carries two survey rows — the
background and the guides — because both are what surrounds the composition
rather than what is in it, and both are settings on the project rather than
nodes on the timeline.

The first wave is the table above.

**The second wave: the whole upstream.**

| arc | rule | from | to | module | command | test | section |
|---|---|---|---|---|---|---|---|
| **13** | the media library is a list of assets, and an asset is added and removed | `docs/plans/artcraft.md` | `craftkit-plugin-art` | `src/media.rs` | `art.media.add`, `art.media.remove` | `crates/craftkit-plugin-art/tests/media.rs` | §1346 |
| **14** | an animatable parameter takes a keyframe, and a keyframe can be moved and removed | `docs/plans/artcraft.md` | `craftkit-plugin-art` | `src/keyframes.rs` | `art.keyframe.upsert`, `art.keyframe.retime`, `art.keyframe.remove` | `crates/craftkit-plugin-art/tests/keyframes.rs` | §1347 |
| **15** | an element carries a mask, and the mask has points, an inversion, a feather and a size | `docs/plans/artcraft.md` | `craftkit-plugin-art` | `src/masks.rs` | `art.mask.point`, `art.mask.invert`, `art.mask.feather`, `art.mask.size` | `crates/craftkit-plugin-art/tests/masks.rs` | §1348 |
| **16** | the composition is exported to a video file, and the file's format and quality are asked for | `docs/plans/artcraft.md` | `craftkit-plugin-film` | `src/export.rs` | `film.export.video` | `crates/craftkit-plugin-film/tests/export.rs` | §1349 |
| **17** | a generation carries per-model parameters, and a price table the kit carries as data gives the estimate before generating | `docs/plans/artcraft.md` | `craftkit-sdk` | `src/prices.rs` | `craftkit-sdk::prices::ModelPrice` | `crates/craftkit-sdk/tests/prices.rs` | §1350 |
| **17** | the estimate is computed from that table, and a model the table has no price for is refused by name | `docs/plans/artcraft.md` | `craftkit-plugin-art` | `src/generate.rs` | `art.generate.parameters`, `art.generate.estimate` | `crates/craftkit-plugin-art/tests/generate.rs` | §1350 |
| **18** | a background is removed from a picture, and a watermark is removed from a picture | `docs/plans/artcraft.md` | `craftkit-plugin-image` | `src/edit.rs` | `image.background.remove`, `image.watermark.remove` | `crates/craftkit-plugin-image/tests/edit.rs` | §1351 |
| **18** | a frame is taken out of a video, and the frame is a picture | `docs/plans/artcraft.md` | `craftkit-plugin-film` | `src/frames.rs` | `film.frame.extract` | `crates/craftkit-plugin-film/tests/frames.rs` | §1351 |
| **18** | the drawing canvas is a front end over the vector commands that already exist, and it adds no command of its own | `docs/plans/artcraft.md` | `craftkit-plugin-vector` | `src/canvas.rs` | `vector.path.create`, `vector.path.freehand` — the commands the page drives | `crates/craftkit-plugin-vector/tests/canvas.rs` | §1351 |
| **18** | the image-to-3D mesh, the storyboard and the moodboard pages are front ends over the art commands that already exist, and they add no command of their own | `docs/plans/artcraft.md` | `craftkit-plugin-art` | `src/pages.rs` | `art.generate`, `art.scene.add` — the commands the pages drive | `crates/craftkit-plugin-art/tests/pages.rs` | §1351 |

The second wave is the rest of the upstream: the six rows the survey held back,
and the pages it skipped. The survey's LATER verdicts were judgements about
size — each of those rows is a model or a dependency before it is a command —
and they were not judgements about scope. Nothing in the upstream's
video-editor surface is out of scope for this plan, and a row that belongs to
another battery is an arc **in** that battery: the export lands in the film
battery, the frame extraction lands in the film battery, the picture edits land
in the image battery, and the drawing canvas lands in the vector battery.

Two of the second wave's arcs carry more than one row, because a row is one
crate. Arc 17 lands in the SDK, where the price table is data, and in the art
battery, where the estimate is computed. Arc 18 lands in four batteries, and
its pages are front ends: the drawing canvas drives the vector commands that
already exist, and the mesh, storyboard and moodboard pages drive the art
commands that already exist, so those rows add a page and not a command.

## 3. The order, and what each arc ships

| arc | ships | cost |
|---|---|---|
| 1 | duplicating an element, animations and all | small — a copy that carries the element's own data |
| 2 | splitting an element at a point in time | medium — a cut point, and two elements where there was one |
| 3 | muting a track | small — a flag on the track, and a payload the track commands do not carry today |
| 4 | hiding a track | small — the same flag, the other way round |
| 5 | removing a track | medium — a removal that takes the track's elements with it |
| 6 | removing a scene | small — the counterpart of the add the battery already has |
| 7 | a scene's bookmarks | medium — a bookmark is a named point in the scene |
| 8 | the effect stack | large — a stack, an order, and a parameter on each entry |
| 9 | the timeline clipboard | medium — a copy buffer, and a paste that places what it copied |
| 10 | the canvas size | small — two numbers and a rule that refuses the bad ones |
| 11 | the project's settings | medium — one record, and the settings the battery already carries |
| 12 | the background and the guides | medium — two settings and the guide kinds the upstream draws |
| 13 | the media library | medium — a list of assets, and the battery's own "a token, not a file" wording giving way to a real asset |
| 14 | keyframes on an element's animatable parameters | large — a time-varying parameter, and the kinds of parameter that may take one |
| 15 | masks on an element | large — a second geometry over an element's own pixels |
| 16 | exporting the composition to a video file | large — an encoder, and the rasteriser's dependency that carries it |
| 17 | per-model generation parameters, and the estimate | medium — a price table as data, and an estimate read from it |
| 18 | the upstream's application pages | medium — four batteries, and pages that are front ends over commands that already exist |

The order follows the survey's order, and the survey's order is the order in
which the capabilities depend on each other. The element arcs come first
because they decide what an element is; the track arcs come next because they
are properties of the tracks those elements sit on; the scene arcs follow,
because a scene is what holds the tracks. The effect stack comes after the
element, because it is a property of an element; the clipboard comes after
both, because it copies what they made. The canvas, the settings and the
background and guides come last, because they describe the composition the
timeline sits in rather than anything on it.

The second wave follows the first, and in the same spirit: the media library
comes before the keyframes, because a keyframe is on a parameter and a
parameter is on an element that may hold media; the masks come after the
keyframes, because a mask is drawn over an element's own pixels; the export
comes after the masks, because the file holds what they made. The generation
arc comes last of the battery's own, because the estimate is read from a table
that changes on its own schedule. The pages come at the end of the whole
wave, because a page is a front end, and a front end is built over commands
that already work.

Arc 16 is the one arc of the wave that is not a command over the document: it
writes a file. The encoders are the tree's rasteriser's dependency, and the
arc names that dependency — `craftkit-raster` gains an encoder library,
`craftkit-video`, that turns a sequence of frames into an mp4 or a webm. The
film battery already draws through the rasteriser, so the export is a command
in that battery over a dependency the rasteriser already carries.

## 4. The tests

One test file per arc, named in the arc's row above, and each one lands with
the arc it belongs to.

- **1** — a duplicated element carries the original's animations; the copy has its own identity; editing one does not edit the other.
- **2** — splitting at a point in time makes two elements that meet at that point; a point outside the element is refused.
- **3** — a muted track says so; unmuting it says so otherwise; muting a track that is not there is refused.
- **4** — a hidden track says so; a hidden track's elements are still on the timeline.
- **5** — a removed track is gone, and its elements are gone with it; removing a track that is not there is refused.
- **6** — a removed scene is gone; removing a scene that is not there is refused.
- **7** — a bookmark is on its scene; toggling it twice leaves it as it was; a bookmark outside the scene is refused.
- **8** — effects are in the order they were added; a removed effect is gone; a reordered stack is the order asked for; a toggled effect is off; a parameter that the effect does not have is refused.
- **9** — a copied element pastes with its animations; a paste pastes at the point asked for; a copy of nothing is refused.
- **10** — the canvas size is the size asked for; a size that is not positive is refused.
- **11** — the settings record holds what was set; the fps and the ticks per second the battery already carried are still there.
- **12** — the background colour is the colour asked for; the blur is the blur asked for; a guide kind that is not one of thirds, grid or safe area is refused.
- **13** — an added asset is in the library; a removed asset is gone; removing an asset that is not there is refused.
- **14** — a keyframe sits at the time asked for, on the parameter asked for; a retimed keyframe moves; a removed keyframe is gone; a keyframe on a parameter that does not take one is refused.
- **15** — a mask's points are the points asked for; the mask is inverted when it is asked to be; the feather and the size are what was asked for; a point outside the element is refused.
- **16** — the exported file is a video file, and it is read back as the format asked for; a quality preset that is not one of the presets is refused.
- **17** — the estimate is the price table's price for the model and the parameters asked for; a model the table has no price for is refused by name; the estimate changes when the parameters change.
- **18** — a removed background is gone and a removed watermark is gone; a frame taken out of a video is the frame at the time asked for; the drawing canvas drives the vector commands and adds no command of its own; the mesh, storyboard and moodboard pages drive the art commands and add none of their own.

## 5. What it does not do

Nothing in the upstream's video-editor surface is refused. Every row the survey
held back is an arc above, and the rows that belonged to another battery are
arcs **in** that battery.

One thing is worth keeping in the prose, because it is the rule that makes the
generation arc honest: the estimate is computed from a price table the kit
carries; the tree makes no outbound call and holds no credential, so the arc
is a rule over data and not a network call.

## 6. The open questions

The upstream's desktop command surface was only sampled by directory, and most
of those commands are account and billing commands, which were judged out of
scope without reading each one.

The upstream's 3-D scene library was not read at all, so the 3-D camera and
posing surface is unreported, and this plan says nothing about it.

The application pages were listed, not read, so those rows are named from
directory names alone and should be checked before any of them is ported.

Whether the tree's existing image, fx and film batteries already cover those
pages is unsure from what the survey read: the skip verdicts rest on their
existence, not on a full comparison of their command lists.

The survey did not say whether the upstream's track commands carry one payload
or several, so arcs 3, 4 and 5 assume one payload each and will need their
readers settled when they land.

The second wave leaves three things open. The encoder is the largest piece of
it: the tree forbids unsafe, so the encoder that arc 16 names has to be the
kit's own, and its shape and its name are not settled. The price table the kit
carries is data somebody has to keep current, and the plan does not say who
does that. And the pages were named from directory names alone, so the rows
that call a page a front end over commands that already exist are a claim
about a surface nobody has read yet, and each page's command list must be read
before its arc lands.
