# The artcraft plan: what the upstream has and the art battery does not

This plan is a survey of one upstream tree against the art battery, written
before any of it is built. It names twelve arcs of future work, in order, and
says plainly what the upstream has that the battery does not.

## 0. The rule

Every arc serves one rule: **a capability the upstream has is a command in the
art battery, and a capability the upstream does not have is not.**

## 1. What the survey found

The survey read the upstream at `/mnt/data/mmacedoeu/work/tools/artcraft` and
compared it with `craftkit-plugin-art` as it stands. Nine capabilities are
marked PORT: they are commands to port. Six are marked LATER: they are whole
models rather than commands, and they are named in §5. One is marked SKIP: the
upstream's own application pages, which belong to other batteries.

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
Each of those is a model or a dependency before it is a command, and a plan
that ported a model in one arc would be a different kind of arc from the twelve
here.

## 2. The arcs

Each arc is three edits: a row in the port manifest, a row in the README's claim
table, and a section in the README. The sections reserved for this plan are
**§1240 through §1251**, one per arc, in the order below.

| arc | rule | from | to | module | command | test | section |
|---|---|---|---|---|---|---|---|
| **1** | an element is duplicated on its own, animations and all | `docs/plans/artcraft.md` | `craftkit-plugin-art` | `src/duplicate.rs` | `art.element.duplicate` | `crates/craftkit-plugin-art/tests/duplicate.rs` | §1240 |
| **2** | an element is split at a point in time | `docs/plans/artcraft.md` | `craftkit-plugin-art` | `src/split.rs` | `art.element.split` | `crates/craftkit-plugin-art/tests/split.rs` | §1241 |
| **3** | a track can be muted, and muting is a flag on the track | `docs/plans/artcraft.md` | `craftkit-plugin-art` | `src/track.rs` | `art.track.mute` | `crates/craftkit-plugin-art/tests/track_mute.rs` | §1242 |
| **4** | a track can be hidden, and hiding is a flag on the track | `docs/plans/artcraft.md` | `craftkit-plugin-art` | `src/track.rs` | `art.track.visibility` | `crates/craftkit-plugin-art/tests/track_visibility.rs` | §1243 |
| **5** | a track can be removed, and removing it takes its elements with it | `docs/plans/artcraft.md` | `craftkit-plugin-art` | `src/track.rs` | `art.track.remove` | `crates/craftkit-plugin-art/tests/track_remove.rs` | §1244 |
| **6** | a scene can be removed, not only added | `docs/plans/artcraft.md` | `craftkit-plugin-art` | `src/scene.rs` | `art.scene.remove` | `crates/craftkit-plugin-art/tests/scene_remove.rs` | §1245 |
| **7** | a scene carries its bookmarks, and a bookmark is toggled | `docs/plans/artcraft.md` | `craftkit-plugin-art` | `src/scene.rs` | `art.scene.bookmark` | `crates/craftkit-plugin-art/tests/scene_bookmark.rs` | §1246 |
| **8** | an element's effects are a stack, and the stack is edited | `docs/plans/artcraft.md` | `craftkit-plugin-art` | `src/effects.rs` | `art.effect.add`, `art.effect.remove`, `art.effect.reorder`, `art.effect.toggle`, `art.effect.set` | `crates/craftkit-plugin-art/tests/effects.rs` | §1247 |
| **9** | elements and keyframes are copied and pasted on the timeline | `docs/plans/artcraft.md` | `craftkit-plugin-art` | `src/clipboard.rs` | `art.clipboard.copy`, `art.clipboard.paste` | `crates/craftkit-plugin-art/tests/clipboard.rs` | §1248 |
| **10** | the composition has a canvas size, and the size is a setting | `docs/plans/artcraft.md` | `craftkit-plugin-art` | `src/canvas.rs` | `art.canvas.size` | `crates/craftkit-plugin-art/tests/canvas.rs` | §1249 |
| **11** | the project's settings are one record, and the record is edited | `docs/plans/artcraft.md` | `craftkit-plugin-art` | `src/project.rs` | `art.project.settings` | `crates/craftkit-plugin-art/tests/project.rs` | §1250 |
| **12** | what is drawn around the composition — the background and the guides — is a setting | `docs/plans/artcraft.md` | `craftkit-plugin-art` | `src/background.rs` | `art.background.color`, `art.background.blur`, `art.guides.set` | `crates/craftkit-plugin-art/tests/background.rs` | §1251 |

These arcs are twelve because the survey marked nine capabilities PORT and three
of them hold more than one command: the track row is mute, visibility and
removal, the scene row is the removal and the bookmarks, and the project
settings row is the canvas size and the rest of the settings. Arc 8 is the
only arc with five command ids, because add, remove, reorder, toggle and set
are five commands over one stack. Arc 12 carries two survey rows — the
background and the guides — because both are what surrounds the composition
rather than what is in it, and both are settings on the project rather than
nodes on the timeline.

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

The order follows the survey's order, and the survey's order is the order in
which the capabilities depend on each other. The element arcs come first
because they decide what an element is; the track arcs come next because they
are properties of the tracks those elements sit on; the scene arcs follow,
because a scene is what holds the tracks. The effect stack comes after the
element, because it is a property of an element; the clipboard comes after
both, because it copies what they made. The canvas, the settings and the
background and guides come last, because they describe the composition the
timeline sits in rather than anything on it.

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

## 5. What it does not do

The five LATER rows are not in this plan, and each is named here with the reason.

- **The media library: add an asset, remove an asset** — it needs a library of real files, and the battery says that media is "a token, not a file"; the library is a model before it is a command.
- **Keyframes on an element's animatable parameters** — it needs a time-varying parameter, and the parameters the battery has are single values.
- **Masks on an element (freeform points, invert, feather, dimensions)** — it needs a mask over an element's own pixels, which is a second geometry beside the one the battery draws.
- **Export of the composition to a video file (mp4, webm, quality presets)** — it needs an encoder, and an encoder is a dependency rather than a command.
- **Per-model generation parameters and a cost estimate before generating** — it needs a model table with per-model parameters and prices, which the upstream keeps in its own client crate.

The SKIP row is not in this plan either: the upstream's named application pages
are other batteries' work, and this plan is about the art battery.

Each of these is a whole model or a new dependency rather than a command, and
this plan is a plan for commands.

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
