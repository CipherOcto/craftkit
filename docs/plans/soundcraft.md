# The soundcraft plan: what the upstream has and the sound battery does not

This plan is a survey of one upstream tree against the sound battery, written
before any of it is built. It names thirty-eight arcs of future work, in order,
and says plainly what the upstream has that the battery does not. The arcs
reserve **§1272 through §1399** of the README, one section each, in order.

## 0. The rule

Every arc serves one rule: **a capability the upstream has is a command in
the battery that owns it, and a capability the upstream does not have is
not.**

## 1. What the survey found

The survey read the upstream's command registry and compared it with
`craftkit-plugin-sound` as it stands. The battery has 144 commands. The
upstream's commands that the battery already has are parity, not gaps, and
they are noted below rather than given an arc. Twenty-four rows are gaps:
either the command is absent, or it exists only as a field inside another
command's payload.

| capability | upstream path:line | craftkit's state | verdict |
|---|---|---|---|
| Invert the phase of a track | `crates/engine/src/cmd/mix.rs:66` | partial: `sound.track.set` carries `phase`, but no command of its own | PORT |
| Trim a track's input | `crates/engine/src/cmd/mix.rs:77` | partial: `sound.track.set` carries `trim_db` | PORT |
| Bypass a plugin | `crates/engine/src/cmd/mix.rs:100` | partial: `sound.insert.set` carries `bypass` | PORT |
| Surround-pan a send | `crates/engine/src/cmd/surround.rs:19` | absent | PORT |
| Copy automation to a send | `crates/engine/src/cmd/mix.rs:176` | absent | PORT |
| Track comments | `crates/engine/src/cmd/track.rs:65` | absent | PORT |
| Track colour | `crates/engine/src/cmd/track.rs:66` | absent | PORT |
| Track folders | `crates/engine/src/cmd/track.rs:43` | partial: the battery groups tracks but has no folder | PORT |
| Track playlists: select and promote | `crates/engine/src/cmd/track.rs:123` | partial: the battery has playlists, but no per-track select or promote | PORT |
| Extract the MIDI of a track | `crates/engine/src/cmd/track_more.rs:52` | partial: `sound.track.midi.import` imports; there is no extract | PORT |
| Clear the material of a range | `crates/engine/src/cmd/edit.rs:22` | absent | PORT |
| Insert silence | `crates/engine/src/cmd/edit.rs:95` | absent (there is `sound.time.insert`, which inserts time on the timeline) | PORT |
| Shift a range later or earlier | `crates/engine/src/cmd/edit.rs:94` | absent | PORT |
| Nudge a range | `crates/engine/src/cmd/edit.rs:140` | absent | PORT |
| Paste special | `crates/engine/src/cmd/edit_more.rs:103` | absent | PORT |
| Copy a selection to another playlist | `crates/engine/src/cmd/edit_more.rs:179` | absent | PORT |
| Set a clip's colour | `crates/engine/src/cmd/clip.rs:117` | absent | PORT |
| Capture a range as a clip | `crates/engine/src/cmd/clip.rs:177` | absent | PORT |
| Regroup clips | `crates/engine/src/cmd/clip_more.rs:13` | absent | PORT |
| Split a note into two | `crates/engine/src/cmd/event_more.rs:81` | absent | PORT |
| Extract the chords of a clip | `crates/engine/src/cmd/event_more.rs:120` | absent | PORT |
| Import audio | `crates/engine/src/cmd/file.rs:62` | partial: `sound.source.add` makes a source of a file; there is no import that reads and registers one | PORT |
| Export the session as text | `crates/engine/src/cmd/file.rs:121` | absent | PORT |
| Session audio health check | `crates/engine/src/cmd/health.rs:11` | absent | PORT |
| Inspect the session, a track or a clip | `crates/engine/src/cmd/query.rs:9` | partial: the battery's kinds are readable, but there is no one inspect command | PORT |
| Place an audio file on a track | `crates/engine/src/cmd/clip.rs:141` | partial: the parts exist, but there is no one command that places a source at a place | PORT |

The twenty-four rows above are the gaps. The twelve arcs of §2 cover twenty of
them; the four rows the survey held back are arcs 13 to 16 of the second wave
in §2: copying automation to a send, the folder of tracks, promoting a playlist
onto a track, and writing a track's MIDI out to a file.

The rest of the upstream's surface is parity: the battery already has a
command for it, and a command the tree has is not an arc. Those rows carry the
same PORT mark in the survey, because the survey's verdict column marks every
capability the upstream has, and it is the state column that says whether the
command is already there.

The survey's five sentences are the shape of the whole comparison. The upstream
has a transport and this battery has none. The upstream has a runtime layer —
automation modes, metering, a playing mixer — and this battery has the
document half and none of the runtime half. The upstream has clip- and
track-level colour, comments, height, hidden state, pin and folders, and this
battery has none of them. The upstream has a family of commands that move a
value between two owners, and this battery has the owners but not the moves.
The upstream has an interchange and an export surface this battery does not
reach.

The runtime half and the export surface are arcs of the second wave in §2. The
rest of the comparison — the facts, the moves, the edits — is what the
twelve arcs are.

## 2. The arcs

Each arc is three edits: a row in the port manifest, a row in the README's claim
table, and a section in the README. The sections reserved for this plan are
**§1272 through §1399**, one per arc, in the order below.

| arc | rule | from | to | module | command | test | section |
|---|---|---|---|---|---|---|---|
| **1** | a track's own facts are facts on the track: the phase, the input trim, the comment and the colour | `docs/plans/soundcraft.md` | `craftkit-plugin-sound` | `src/track.rs` | `sound.track.phase`, `sound.track.trim`, `sound.track.comment`, `sound.track.color` | `crates/craftkit-plugin-sound/tests/track.rs` | §1272 |
| **2** | bypassing is a flag on the insert, and not a removal of it | `docs/plans/soundcraft.md` | `craftkit-plugin-sound` | `src/mixer.rs` | `sound.insert.bypass` | `crates/craftkit-plugin-sound/tests/insert.rs` | §1273 |
| **3** | a send carries a surround pan the same way a track does | `docs/plans/soundcraft.md` | `craftkit-plugin-sound` | `src/surround.rs` | `sound.send.surround` | `crates/craftkit-plugin-sound/tests/surround.rs` | §1274 |
| **4** | a range has its own operations: clearing it, shifting it, nudging it | `docs/plans/soundcraft.md` | `craftkit-plugin-sound` | `src/ranges.rs` | `sound.range.clear`, `sound.range.shift`, `sound.range.nudge` | `crates/craftkit-plugin-sound/tests/ranges.rs` | §1275 |
| **5** | silence is a range with no material in it | `docs/plans/soundcraft.md` | `craftkit-plugin-sound` | `src/time.rs` | `sound.time.insert.silence` | `crates/craftkit-plugin-sound/tests/silence.rs` | §1276 |
| **6** | the material in the clipboard may be moved to another place in special ways | `docs/plans/soundcraft.md` | `craftkit-plugin-sound` | `src/clipboard.rs` | `sound.clipboard.paste.merge`, `sound.playlist.copy.to` | `crates/craftkit-plugin-sound/tests/clipboard.rs` | §1277 |
| **7** | a clip's own facts are facts on the clip: the colour, the capture, the group | `docs/plans/soundcraft.md` | `craftkit-plugin-sound` | `src/clip.rs` | `sound.clip.color`, `sound.clip.capture`, `sound.clip.regroup` | `crates/craftkit-plugin-sound/tests/clip.rs` | §1278 |
| **8** | the notes of a clip are rules over the notes | `docs/plans/soundcraft.md` | `craftkit-plugin-sound` | `src/midi.rs` | `sound.clip.midi.note.split`, `sound.clip.midi.chords` | `crates/craftkit-plugin-sound/tests/midinotes.rs` | §1279 |
| **9** | the session is read from outside: a file is imported, the session is written out, the media is checked, the document is inspected | `docs/plans/soundcraft.md` | `craftkit-plugin-sound` | `src/session.rs` | `sound.source.import`, `sound.session.text`, `sound.health.read`, `sound.session.inspect` | `crates/craftkit-plugin-sound/tests/session.rs` | §1280 |
| **10** | placing is a source, a track and a place, and the three parts are already commands | `docs/plans/soundcraft.md` | `craftkit-plugin-sound` | `src/clip.rs` | `sound.clip.place` | `crates/craftkit-plugin-sound/tests/place.rs` | §1281 |
| **11** | what the ruler and the counter show is a fact of the session, and the session says it | `docs/plans/soundcraft.md` | `craftkit-plugin-sound` | `src/view.rs` | `sound.ruler.set`, `sound.counter.set` | `crates/craftkit-plugin-sound/tests/view.rs` | §1282 |
| **12** | the saved views and the window's own state are facts of the session: the column views, the clip display options, the scrolling mode, the solo mode and the keyboard focus | `docs/plans/soundcraft.md` | `craftkit-plugin-sound` | `src/session.rs` | `sound.view.save`, `sound.view.list`, `sound.session.options.set` | `crates/craftkit-plugin-sound/tests/views.rs` | §1283 |

These arcs are twelve because the survey's twenty-four gaps fall into ten
groups by owner, and the owner's decision about the view and the options adds
two more. The track's own facts are one arc, because the phase, the trim, the
comment and the colour are all facts about one track and each of them is a
field the battery's track command already carries in one form. The insert's
bypass is one arc on its own, because it is a flag on an insert and the
insert is a different owner. The send's surround pan is one arc, because a
send is a third owner and the battery's surround commands are about tracks.

The range's operations are one arc — clear, shift, nudge — because all three
are edits over the same range and they share one notion of what a range is.
Silence is an arc of its own, because it is an insertion and not an edit of
what is already there, and because the battery already has an insertion of
time that it must not be confused with. The material moved to another place is
one arc, because a special paste and a copy into another playlist are the same
move with different rules over the same clipboard.

The clip's own facts are one arc: the colour, the capture and the regroup are
all about a clip, and a group is already a name over a set of clips in the
battery. The notes of a clip are one arc, because a split and a chord
extraction are both rules over the notes of one MIDI clip.

The session read from outside is one arc of four commands, because all four
read or register the session rather than edit it: importing a file, writing the
session out as text, checking that the media is still there, and inspecting
what the document holds. The placement of a source is an arc of its own,
because placing composes three existing commands into one and is therefore
the only arc here that is a composition rather than a new fact.

The four rows the survey held back are not held back now: they are arcs 13 to 16
of the second wave below. Copying automation to a send
(`crates/engine/src/cmd/mix.rs:176`), the folder of tracks
(`crates/engine/src/cmd/track.rs:43`), promoting a playlist onto a track
(`crates/engine/src/cmd/track.rs:123`), and writing a track's MIDI out
(`crates/engine/src/cmd/track_more.rs:52`). Each of them needs a model the
battery does not yet have — a lane that can be copied onto a send, a folder
beside a group, a per-track playlist selection, and a file written out of a
document that holds notes — and each arc builds that model first and writes the
command second.

**Arcs 11 and 12 came from the owner's word and not from the survey.** The
survey read the ruler, the counter, the column views, the clip display
options and the scrolling, solo and focus modes as a front end's business and
held them back. The owner has said the saved view and option state belongs in
the session, so they are arcs now: the ruler and the counter are one arc,
because both are ways of showing a number of the timeline, and the saved views
with the window's own state are one arc, because a saved view and the mode the
window is in are both the session saying how it is to be looked at.

**The second wave: the whole upstream.**

| arc | rule | from | to | module | command | test | section |
|---|---|---|---|---|---|---|---|
| **13** | a lane of automation may be copied from one owner to another, and the copy is a lane of its own on the second owner | `docs/plans/soundcraft.md` | `craftkit-plugin-sound` | `src/automation.rs` | `sound.send.automation.copy` | `crates/craftkit-plugin-sound/tests/automation.rs` | §1374 |
| **14** | a folder is a group of tracks that the session names, and the name is the fact | `docs/plans/soundcraft.md` | `craftkit-plugin-sound` | `src/folder.rs` | `sound.track.folder` | `crates/craftkit-plugin-sound/tests/folders.rs` | §1375 |
| **15** | a track may name one of the session's playlists as its own, and promoting it makes that playlist the track's | `docs/plans/soundcraft.md` | `craftkit-plugin-sound` | `src/playlist.rs` | `sound.track.playlist.select`, `sound.track.playlist.promote` | `crates/craftkit-plugin-sound/tests/playlist.rs` | §1376 |
| **16** | a track's notes may be written out to a file, and the file is a MIDI file | `docs/plans/soundcraft.md` | `craftkit-plugin-sound` | `src/midi.rs` | `sound.track.midi.extract` | `crates/craftkit-plugin-sound/tests/midextract.rs` | §1377 |
| **17** | the modes of the automation lanes are facts of the session, and the session says which mode a lane is in | `docs/plans/soundcraft.md` | `craftkit-plugin-sound` | `src/automationmodes.rs` | `sound.automation.mode.set` | `crates/craftkit-plugin-sound/tests/automationmodes.rs` | §1378 |
| **18** | the state of a plug-in is kept in the session, and the session says what it is | `docs/plans/soundcraft.md` | `craftkit-plugin-sound` | `src/plugin.rs` | `sound.plugin.state.set` | `crates/craftkit-plugin-sound/tests/pluginstate.rs` | §1379 |
| **19** | an instrument on a track is a fact of the track, and the track says which instrument it carries | `docs/plans/soundcraft.md` | `craftkit-plugin-sound` | `src/instrument.rs` | `sound.track.instrument.set` | `crates/craftkit-plugin-sound/tests/instrument.rs` | §1380 |
| **20** | a sampler is a source that plays a sample, and the sample it plays is named by the source | `docs/plans/soundcraft.md` | `craftkit-plugin-sound` | `src/sampler.rs` | `sound.sampler.load` | `crates/craftkit-plugin-sound/tests/sampler.rs` | §1381 |
| **21** | pre-roll is a length of time the session plays before the playhead, and the session says what it is | `docs/plans/soundcraft.md` | `craftkit-plugin-sound` | `src/preroll.rs` | `sound.preroll.set` | `crates/craftkit-plugin-sound/tests/preroll.rs` | §1382 |
| **22** | the volume automation of a clip may be turned into the clip's gain, and the lanes it replaces are gone | `docs/plans/soundcraft.md` | `craftkit-plugin-sound` | `src/clipgain.rs` | `sound.automation.to_clip_gain` | `crates/craftkit-plugin-sound/tests/clipgain.rs` | §1383 |
| **23** | a track's timebase is a fact of the track, and the track says what beats it counts in | `docs/plans/soundcraft.md` | `craftkit-plugin-sound` | `src/timebase.rs` | `sound.track.timebase.set` | `crates/craftkit-plugin-sound/tests/timebase.rs` | §1384 |
| **24** | a track preset is a named set of the track's facts, and applying one writes those facts | `docs/plans/soundcraft.md` | `craftkit-plugin-sound` | `src/preset.rs` | `sound.track.preset.apply` | `crates/craftkit-plugin-sound/tests/preset.rs` | §1385 |
| **25** | the gain of a clip may be rendered into the clip's audio, and the render is the audio that is played | `docs/plans/soundcraft.md` | `craftkit-plugin-sound` | `src/gain.rs` | `sound.clip.gain.render` | `crates/craftkit-plugin-sound/tests/gain.rs` | §1386 |
| **26** | a warp may be removed from a clip, and the clip holds no warped material afterwards | `docs/plans/soundcraft.md` | `craftkit-plugin-sound` | `src/warp.rs` | `sound.clip.warp.remove` | `crates/craftkit-plugin-sound/tests/warp.rs` | §1387 |
| **27** | a pitch shift may be removed from a clip, and the clip holds no shifted material afterwards | `docs/plans/soundcraft.md` | `craftkit-plugin-sound` | `src/pitch.rs` | `sound.clip.pitch.remove` | `crates/craftkit-plugin-sound/tests/pitch.rs` | §1388 |
| **28** | an edit mode is a fact of the session, and the session says which mode the edits are made in | `docs/plans/soundcraft.md` | `craftkit-plugin-sound` | `src/editmode.rs` | `sound.edit.mode.set` | `crates/craftkit-plugin-sound/tests/editmode.rs` | §1389 |
| **29** | a tempo curve is a fact of the session, and the session says where the tempo changes | `docs/plans/soundcraft.md` | `craftkit-plugin-sound` | `src/tempo.rs` | `sound.tempo.curve.set` | `crates/craftkit-plugin-sound/tests/tempo.rs` | §1390 |
| **30** | step input writes notes on a grid, and the grid is the session's | `docs/plans/soundcraft.md` | `craftkit-plugin-sound` | `src/step.rs` | `sound.step.input` | `crates/craftkit-plugin-sound/tests/step.rs` | §1391 |
| **31** | a performance may be restored, and restoring it puts the session back as it was when it was taken | `docs/plans/soundcraft.md` | `craftkit-plugin-sound` | `src/performance.rs` | `sound.performance.restore` | `crates/craftkit-plugin-sound/tests/performance.rs` | §1392 |
| **32** | audio may be read as MIDI, and the notes it gives are the notes the session holds | `docs/plans/soundcraft.md` | `craftkit-plugin-sound` | `src/audiomidi.rs` | `sound.audio.to_midi` | `crates/craftkit-plugin-sound/tests/audiomidi.rs` | §1393 |
| **33** | an interchange file is read and analysed, and the analysis says what the file holds | `docs/plans/soundcraft.md` | `craftkit-plugin-sound` | `src/interchange.rs` | `sound.interchange.analyze` | `crates/craftkit-plugin-sound/tests/interchange.rs` | §1394 |
| **34** | the setup of the session and the devices it uses are facts of the session, and the session says what they are | `docs/plans/soundcraft.md` | `craftkit-plugin-sound` | `src/setup.rs` | `sound.setup.set`, `sound.device.list` | `crates/craftkit-plugin-sound/tests/setup.rs` | §1395 |
| **35** | the metering of the session is read through a command, and the command answers with the levels it read | `docs/plans/soundcraft.md` | `craftkit-plugin-sound` | `src/meter.rs` | `sound.meter.read` | `crates/craftkit-plugin-sound/tests/meter.rs` | §1396 |
| **36** | the transport plays and stops the session, and the transport is a fact about where the playhead is | `docs/plans/soundcraft.md` | `craftkit-plugin-film` | `src/transport.rs` | `film.transport.play`, `film.transport.stop` | `crates/craftkit-plugin-film/tests/transport.rs` | §1397 |
| **37** | a shortcut names a command, and a shortcut that names a command the tree does not have is refused by that name | `docs/plans/soundcraft.md` | `craftkit-sdk` | `src/shortcut.rs` | `sdk.shortcut.bind` | `crates/craftkit-sdk/tests/shortcut.rs` | §1398 |
| **38** | the zoom of the view is the camera's, and the camera fits the whole and fits a selection | `docs/plans/soundcraft.md` | `craftkit-sdk` | `src/camera.rs` | `sdk.camera.fit`, `sdk.camera.fit_selection` | `crates/craftkit-sdk/tests/camera.rs` | §1399 |

The second wave is the rest of the upstream. Every capability the upstream has
that the sound battery does not, is an arc above, and the plan is **thirty-eight
arcs** in all: twelve in the first wave and twenty-six in the second.

The survey's "later" verdicts were judgements about **size**, not about scope.
A row was marked later because the model it needs was larger than a field — a
lane that can be copied onto a send, a device, a running mixer — and not
because the capability was outside the plan. The owner has said the family is
fully ported, so every one of those rows is an arc, and the size of a model is
a cost in §3 and not a reason to refuse.

A row that belongs to another battery is an arc **in** that battery, and it
keeps that battery's crate. The transport (arc 36) is the film battery's,
because the playhead is the film battery's and the transport is a fact about
where the playhead is. The keyboard shortcuts (arc 37) and the zoom (arc 38)
are the SDK's, because the camera is the tree's one view type with its own
caps, and because the mapping from a key to a command is a fact about the
session and the registry rather than about sound.

## 3. The order, and what each arc ships

| arc | ships | cost |
|---|---|---|
| 1 | a track's own facts, each as a command of its own | small — four fields the track command already carries |
| 2 | the insert's bypass flag | small — a flag and a reader |
| 3 | a send's surround pan | small — the same pan a track has |
| 4 | the range's own operations | medium — three edits that each move or remove material |
| 5 | silence in a range | medium — an insertion that leaves a hole and a length |
| 6 | the special pastes and the copy into another playlist | medium — a clipboard that is read by rule, not pasted whole |
| 7 | the clip's own facts | medium — a colour, a capture and a move between groups |
| 8 | the notes of a clip | medium — a split and a read of the chords |
| 9 | the session read from outside | medium — a file reader, a text writer, a check and an inspect |
| 10 | the placement of a source | small — a composition of three commands that already exist |
| 11 | the ruler and the counter the session shows | small — two names for how a number is written |
| 12 | the saved views and the window's own state | medium — a saved view is a set of facts that can be named, and the modes are fields of the session |
| 13 | a lane of automation copied from one owner onto another | medium — a lane that is read on one owner and written on another |
| 14 | the folder of tracks | medium — a group beside a group, and a name on it |
| 15 | a track's playlist selected and promoted | medium — a second set of clips per track |
| 16 | a track's MIDI written out to a file | medium — a writer of a file out of a document of notes |
| 17 | the modes of the automation lanes | medium — a state machine over the lanes |
| 18 | the plug-in state kept in the session | medium — a state per plug-in, and a place to keep it |
| 19 | an instrument on a track | medium — a device named by the track |
| 20 | a sampler | medium–large — a source that plays a sample |
| 21 | pre-roll | small — a length the session plays before the playhead |
| 22 | volume automation turned into clip gain | medium — a conversion that removes the lanes it replaces |
| 23 | a track's timebase | medium — the beats a track counts in |
| 24 | a track preset | medium — a named set of the track's facts |
| 25 | a clip's gain rendered into the audio | large — a render of the audio as it is played |
| 26 | a warp removed | small — the removal of what the warp made |
| 27 | a pitch shift removed | small — the removal of what the shift made |
| 28 | an edit mode | small — a mode the edits are made in |
| 29 | tempo curves | medium — a curve of the tempo over the timeline |
| 30 | step input | medium–large — a grid and the notes written on it |
| 31 | a performance restored | medium — a taken state put back |
| 32 | audio read as MIDI | medium–large — a reading of audio into notes |
| 33 | an interchange file analysed | large — a reader of somebody else's file format |
| 34 | the setup and the devices | medium — the settings and the devices the session uses |
| 35 | metering | medium–large — a running mixer read through a command |
| 36 | the transport | medium — the play and the stop of a session |
| 37 | the keyboard shortcuts | small — a mapping from a key to a command |
| 38 | the zoom, fitted and fitted to a selection | small — two fits on the tree's one camera |

The order follows the owners, from the top of the session downwards. The
track's facts come first, because a track is what the other owners hang from,
and because two of the four commands of arc 1 are the fields the battery's
existing track command already carries. The insert's flag and the send's pan
follow, because they are the two other owners that carry a setting of their
own.

The range's operations come next, because they need a range to be a range, and
the silence follows them, because it is the same range with nothing in it. The
clipboard arc comes after the ranges, because a special paste is a rule over
material that has already been copied out of one. The clip's own facts come
after that, because a capture is a range turned into a clip and a regroup is a
move between groups the battery already has. The notes of a clip come after the
clip, because a note lives in a MIDI clip.

The session read from outside comes near the end, because it is a read of the
whole document and it is worth reading once the document holds everything the
arcs above put in it. The placement of a source comes after it, because it is
a composition of commands that already exist and nothing else depends on it.

**The two view arcs come last of all**, because they are about how the
document is *looked at* and not about what it holds. They are also the two
arcs the owner asked for by name, so they are written down and priced.

**The second wave follows the same order.** Arcs 13 to 16 are the four rows the
survey held back, and they come first because each of them is a model before
it is a command. Arcs 17 to 35 are the runtime half and the export surface,
and they come in the order of the owner they belong to: the automation lanes,
the plug-ins, the devices, the session's own settings, and last the reads that
look at the whole document. Arcs 36 to 38 are the three rows that belong to
another crate — the transport to the film battery, the shortcuts and the zoom
to the SDK — and they come last because the other crates' arcs are what they
read.

## 4. The tests

One test file per arc, named in the arc's row above, and each one lands with
the arc it belongs to.

- **1** — a track's phase, trim, comment and colour are what was set; a phase that is not a sign is refused; a track that is not there is refused.
- **2** — a bypassed insert says so and stays on the chain; un-bypassing it says so otherwise; bypassing an insert that is not there is refused.
- **3** — a send's surround pan is what was set; a send that is not there is refused.
- **4** — a cleared range holds no material; a shifted range's material moved and the rest did not; a nudged range moved by one step of the current unit; a range that is not there is refused.
- **5** — the silence is the length asked for and holds no material; the material around it stays where it was.
- **6** — a merged paste keeps what was on the timeline and adds what was copied; a copy into another playlist puts the material in that playlist and not in the one it came from.
- **7** — a clip's colour is what was set; a capture makes a clip of the range's material and leaves the range as it was; a regroup moves the clips from one group into another and empties the first.
- **8** — a split note makes two notes that meet at the split point; the chords of a clip are the notes that fall together; a note that is not there is refused.
- **9** — an imported file becomes a source that is registered; the text written out holds the session's facts; a health check reports a missing file and changes nothing; an inspect answers with what the document holds and writes nothing.
- **10** — a placed source sits at the place asked for, on the track asked for; a place that is not a place is refused.
- **11** — the ruler and the counter are what was set; a format the battery does not know is refused by name.
- **12** — a saved view is what was saved and is read back the same; a saved view that is not there is refused; the session's modes are what was set.
- **13** — the copied lane is on the send and reads as the lane it came from; the lane it came from is unchanged.
- **14** — a folder holds the tracks put into it; a track that is not there is refused; a folder with no name is refused.
- **15** — the playlist a track selects is the one that was selected; promoting it makes it the track's; a playlist that is not there is refused.
- **16** — the file written holds the notes of the track; a track that is not there is refused.
- **17** — the mode of a lane is the one that was set; a mode the battery does not know is refused by name.
- **18** — the plug-in's state is what was set and reads back the same; a plug-in that is not there is refused.
- **19** — the instrument on the track is the one that was set; an instrument the tree does not have is refused by name.
- **20** — the sampler plays the sample it was loaded with; a sample that is not there is refused.
- **21** — the pre-roll is the length that was set; a length that is not a length is refused.
- **22** — the clip's gain carries what the volume automation said, and the lanes the conversion replaced are gone.
- **23** — the track's timebase is the one that was set; a timebase the battery does not know is refused by name.
- **24** — applying a preset writes the facts it holds; a preset that is not there is refused.
- **25** — the rendered audio is what the clip plays with its gain, and the clip's gain is still the gain it was.
- **26** — the clip holds no warped material afterwards, and the rest of the clip is as it was.
- **27** — the clip holds no shifted material afterwards, and the rest of the clip is as it was.
- **28** — the edit mode is the one that was set; a mode the battery does not know is refused by name.
- **29** — the tempo curve says where the tempo changes; a point that is not on the timeline is refused.
- **30** — the notes step input writes are on the grid; a grid that is not there is refused.
- **31** — the session after a restore is the session as it was when the performance was taken.
- **32** — the notes the reading gives are the notes the session holds; audio that is not there is refused.
- **33** — the analysis says what the file holds; a file the battery cannot read is refused by name.
- **34** — the setup is what was set; the devices listed are the devices the session uses; a device that is not there is refused.
- **35** — the metering reads the levels it says; a track that is not there is refused.
- **36** — the transport plays and stops, and the playhead is where it was left; a failure restores the session as it was.
- **37** — a shortcut that names a command the tree has is bound; a shortcut that names a command the tree does not have is refused by that name; a key bound twice is refused.
- **38** — the fit shows the whole, and the fit to a selection shows the selection; a selection that is empty is refused.

## 5. What it does not do

The plan refuses one thing, and it names the parity rows.

- **The score printing** is refused: a print surface is not a rule about a document, and the tree's answer to printing is the PDF battery's write.
- **Parity, named in one line**: undo, redo and selection (the core owns them); plug-in discovery (the plug-in batteries own it); pictures and video (the film battery owns them, and the effectcraft plan's second wave ports the media rows there); saving and opening the session (the kit owns it).

Everything else the upstream has is an arc in §2: the first wave's twelve and the
second wave's twenty-six. No row is left out of the plan, and no row is held
back for size.

## 6. The open questions

The survey read the upstream's command registry and not its handler bodies, so
for every upstream row it knows what the command asks for and not the
arithmetic behind it. The rules in §2 are therefore the rules the survey read,
and the arithmetic behind them is still to be read.

A few upstream ids are cited at the line where the macro opens, which is one
line above the id literal. The row names the command; the line is one off.

The survey could not decide whether some upstream capabilities duplicate this
tree's other batteries, because it read only their ids and not their rules. The
batteries it compared with are the film battery and the plugin batteries, and
whether a row is a gap or somebody else's command is still open.

The view and options families were classified out of scope on the tree's own
rules. **The owner has since said the saved view and option state belongs in
the session**, so that question is closed: the ruler, the counter, the saved
views, the clip display options and the scrolling, solo and focus modes are
arcs 11 and 12. What is still open is the **shape**: a saved view is a set of
facts under a name, and whether the battery stores one as a node of its own
kind or as a map of fields is a design choice this plan does not settle.

The edit-mode family was judged later rather than out of scope, and the
survey is not certain of that judgement; it is arc 28 of the second wave.

The survey did not check whether one command is the intended home for a
track's trim and phase, or whether each should be a command of its own. Arc 1
takes them as commands of their own, on the reading that a command is what an
agent calls.

**The view and the options are in scope, by the owner's word.** The survey
held the upstream's `view` and `options` families out of scope, on the tree's
own rule that a front end reads the kit's state and never writes the
document. The owner has said that the saved view and option state belongs in
the session, and it is therefore **two more arcs** — the ruler and the counter
(arc 11) and the saved views with the window's own state (arc 12). The house
rule is not broken by this: the **fact** lives in the document and the battery
owns it, and the front end still reads it and applies it. A front end that
writes the fact is still refused by the gate.

The second wave leaves open the **shape** of the runtime models it names: the
automation modes are a state machine over the lanes, the instrument and the
sampler are devices, the metering is a running mixer, and the interchange
analysis is a reader of somebody else's file format. The plan gives each of
them an arc and a place to land and does not settle how each is written. It
leaves open too the boundary between the batteries: a row that belongs to
another battery is an arc **in** that battery, and the second wave's three such
rows — the transport, the shortcuts and the zoom — name the crate that owns
them.
